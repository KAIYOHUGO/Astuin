use std::collections::BTreeMap;

use crate::Component;
use anyhow::Result;
use edtui::{EditorEventHandler, EditorMode, EditorState, EditorView, Highlight, Index2, Lines};
use ratatui::{crossterm::event::Event, prelude::*};

pub struct CodeComponent {
    state: EditorState,
    event_handler: EditorEventHandler,

    /// end / start,index
    pub tree_lookup: BTreeMap<(usize, usize), (Index2, usize)>,

    pub cursor: Index2,
    tree: usize,
}

impl Default for CodeComponent {
    fn default() -> Self {
        Self {
            state: EditorState::new(Lines::from("")),
            event_handler: EditorEventHandler::vim_mode(),
            tree_lookup: Default::default(),
            cursor: Default::default(),
            tree: 0,
        }
    }
}

impl CodeComponent {
    pub fn new(code: Self, pos_lookup: &[crate::ast::Pos]) -> Self {
        let mut tree_lookup = BTreeMap::new();

        for (i, pos) in pos_lookup.into_iter().enumerate() {
            let crate::ast::Pos::Span(s, e) = pos else {
                continue;
            };
            tree_lookup.insert((e.row, e.col), (*s, i));
        }
        Self {
            tree_lookup,
            ..code
        }
    }

    pub fn code(&self) -> Vec<char> {
        self.state.lines.flatten(&Some('\n'))
    }

    pub fn mode(&self) -> EditorMode {
        self.state.mode
    }

    pub fn selected_tree(&mut self) -> usize {
        if self.cursor == self.state.cursor {
            return self.tree;
        }

        self.cursor = self.state.cursor;
        let cursor = (self.cursor.row, self.cursor.col);
        let res = self
            .tree_lookup
            .range(cursor..)
            .map(|(e, (s, tree))| ((s.row, s.col), e, tree))
            .filter(|(s, ..)| *s <= cursor)
            .take(10)
            .max_by_key(|(s, ..)| *s);

        if let Some((s, e, tree)) = res {
            self.tree = *tree;
            self.highlight(Index2::new(s.0, s.1), Index2::new(e.0, e.1));
        }

        self.tree
    }

    fn highlight(&mut self, s: Index2, mut e: Index2) {
        self.state.clear_highlights();
        e.col = e.col.saturating_sub(1);
        self.state
            .add_highlight(Highlight::new(s, e, Style::new().black().on_light_green()));
    }

    pub fn highlight_and_move_cursor(&mut self, s: Index2, e: Index2) {
        self.state.cursor = s;
        self.highlight(s, e);
    }
}

impl Component for CodeComponent {
    fn on_event(&mut self, event: impl Into<Event>) -> Result<()> {
        self.event_handler.on_event(event.into(), &mut self.state);
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let editor = EditorView::new(&mut self.state)
            .theme(edtui::EditorTheme::default().base(Style::new().white()));
        frame.render_widget(editor, area);
        Ok(())
    }
}
