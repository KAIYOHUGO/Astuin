use anyhow::Result;
use edtui::Index2;
use ratatui::{
    crossterm::event::{Event, KeyCode, KeyModifiers, MouseEventKind},
    prelude::*,
    widgets::ScrollbarOrientation,
};
use sise::TreeNode;
use tui_tree_widget::{Scrollbar, Tree, TreeItem, TreeState};

use crate::Component;

type Ident = usize;
pub struct AstComponent {
    pub state: TreeState<Ident>,
    items: [TreeItem<'static, Ident>; 1],
    pos_lookup: Vec<Pos>,

    span: (Index2, Index2),
}

#[derive(Debug, Clone, Copy)]
pub enum Pos {
    Span(Index2, Index2),
    Ptr(Ident),
}

impl AstComponent {
    pub fn new(tree: TreeNode, extra_output: Vec<String>) -> Self {
        fn helper(
            pos_lookup: &mut Vec<Pos>,
            parent: &mut TreeItem<'static, Ident>,
            tree: TreeNode,
        ) {
            match tree {
                TreeNode::Atom(text) => {
                    let id = pos_lookup.len();
                    pos_lookup.push(Pos::Ptr(*parent.identifier()));

                    parent.add_child(TreeItem::new_leaf(id, text)).unwrap();
                }
                TreeNode::List(tree) => {
                    let mut iter = tree.into_iter();
                    match iter.next() {
                        Some(TreeNode::Atom(text)) if text.as_str() == "@span" => {
                            let mut get_num = move || {
                                iter.next()
                                    .map(|s| s.as_atom().map(|s| s.parse::<usize>().ok()))
                                    .flatten()
                                    .flatten()
                            };
                            let (Some(sl), Some(sc), Some(el), Some(ec)) =
                                (get_num(), get_num(), get_num(), get_num())
                            else {
                                return;
                            };

                            pos_lookup[*parent.identifier()] =
                                Pos::Span(Index2::new(sl, sc), Index2::new(el, ec));
                        }
                        Some(TreeNode::Atom(text)) => {
                            let id = pos_lookup.len();
                            pos_lookup.push(Pos::Ptr(*parent.identifier()));

                            let mut item = TreeItem::new(id, text, vec![]).unwrap();
                            for node in iter {
                                helper(pos_lookup, &mut item, node);
                            }
                            parent.add_child(item).unwrap();
                        }
                        _ => {
                            let id = pos_lookup.len();
                            pos_lookup.push(Pos::Ptr(*parent.identifier()));

                            let text = Text::styled("Unknown Node", Style::new().red());
                            parent.add_child(TreeItem::new_leaf(id, text)).unwrap();
                        }
                    }
                }
            }
        }

        let mut pos_lookup = vec![];

        let id = pos_lookup.len();
        pos_lookup.push(Pos::Span(Default::default(), Default::default()));
        let mut root = TreeItem::new(id, "root", vec![]).unwrap();

        let extra = {
            let extra_id = pos_lookup.len();
            pos_lookup.push(Pos::Ptr(id));
            let mut extra = TreeItem::new(extra_id, "extra output", vec![]).unwrap();

            let output_id = pos_lookup.len();
            pos_lookup.push(Pos::Ptr(id));
            extra
                .add_child(TreeItem::new_leaf(
                    output_id,
                    Text::from_iter(extra_output.into_iter()),
                ))
                .unwrap();
            extra
        };
        root.add_child(extra).unwrap();

        helper(&mut pos_lookup, &mut root, tree);

        let mut state = TreeState::default();
        state.select(vec![id]);
        Self {
            state,
            items: [root],
            pos_lookup,
            span: Default::default(),
        }
    }

    pub fn select(&mut self, id: Ident) {
        fn helper(
            items: &[TreeItem<'static, Ident>],
            list: &mut Vec<Ident>,
            id: Ident,
            state: &mut TreeState<Ident>,
        ) -> Option<()> {
            // i know O(n) is bad
            // but comeon, it's a ast
            let item = items.into_iter().filter(|x| *x.identifier() <= id).last()?;

            let item_id = *item.identifier();
            list.push(item_id);
            state.open(list.clone());
            if item_id == id {
                return Some(());
            }

            helper(item.children(), list, id, state)
        }
        let mut list = vec![];
        let res = helper(self.items.as_slice(), &mut list, id, &mut self.state);
        if res.is_some() {
            self.state.select(list.clone());
        }
    }

    pub fn selected_span(&self) -> (Index2, Index2) {
        self.span
    }

    fn update_span(&mut self) {
        let pos = self
            .state
            .selected()
            .last()
            .copied()
            .map(|x| self.pos_lookup[x]);
        let Some(mut pos) = pos else {
            return Default::default();
        };

        loop {
            match pos {
                Pos::Span(s, e) => {
                    self.span = (s, e);
                    return;
                }
                Pos::Ptr(i) => pos = self.pos_lookup[i],
            }
        }
    }

    pub fn pos_lookup(&self) -> &[Pos] {
        &self.pos_lookup
    }
}

const SCROLL_SPEED: usize = 5;
impl Component for AstComponent {
    fn on_event(&mut self, event: impl Into<Event>) -> Result<()> {
        let event = event.into();
        match event {
            Event::Key(key) => match key.code {
                KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.state.scroll_down(SCROLL_SPEED);
                }
                KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.state.scroll_up(SCROLL_SPEED);
                }

                KeyCode::Char('f') => {
                    self.state.close_all();
                }

                KeyCode::Char('h') | KeyCode::Left => {
                    self.state.key_left();
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    if self.state.key_down() {
                        self.update_span();
                    }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    if self.state.key_up() {
                        self.update_span();
                    }
                }
                KeyCode::Char('l') | KeyCode::Right => {
                    self.state.key_right();
                }
                KeyCode::Char(' ') => {
                    self.state.toggle_selected();
                }
                _ => {}
            },
            Event::Mouse(mouse) => match mouse.kind {
                MouseEventKind::Down(_) => {
                    if self.state.click_at(Position::new(mouse.column, mouse.row)) {
                        self.update_span();
                    }
                }
                MouseEventKind::ScrollDown => {
                    self.state.scroll_down(1);
                }
                MouseEventKind::ScrollUp => {
                    self.state.scroll_up(1);
                }
                _ => {}
            },
            _ => {}
        }
        let _ = event;
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let tree = Tree::new(self.items.as_slice())?
            .experimental_scrollbar(Some(
                Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(None)
                    .track_symbol(None)
                    .end_symbol(None),
            ))
            .highlight_style(
                Style::new()
                    .fg(Color::Black)
                    .bg(Color::LightGreen)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("*")
            .node_open_symbol("-")
            .node_closed_symbol("+")
            .node_no_children_symbol(" ");
        frame.render_stateful_widget(tree, area, &mut self.state);
        // frame.render_widget(Paragraph::new(format!("{:?}", self.items)), area);
        Ok(())
    }
}
