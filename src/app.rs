use std::io::Write;
use std::{mem, thread};

use anyhow::Result;
use edtui::EditorMode;
use ratatui::crossterm::event::{self, Event, KeyModifiers};
use ratatui::prelude::*;
use ratatui::{DefaultTerminal, widgets};
use run_script::ScriptOptions;
use run_script::types::IoOptions;

use crate::Component;
use crate::ast::AstComponent;
use crate::code::CodeComponent;

pub struct App {
    exit: bool,
    cmd: String,

    focus: Focus,
    ast: Result<AstComponent, String>,
    code: CodeComponent,

    ast_block: Rect,
    code_block: Rect,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Code,
    Ast,
}

impl App {
    pub fn new(cmd: String) -> Self {
        Self {
            exit: false,
            cmd,
            focus: Focus::Code,
            ast: Err("No input".to_owned()),
            code: CodeComponent::default(),
            ast_block: Default::default(),
            code_block: Default::default(),
        }
    }
    pub fn run(&mut self, term: &mut DefaultTerminal) -> Result<()> {
        while !self.exit {
            // FIXME: remove unwrap
            term.draw(|frame| self.draw(frame).unwrap())?;
            self.event()?;
        }
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) -> Result<()> {
        let layout = Layout::vertical([Constraint::Length(1), Constraint::Fill(1)])
            .margin(1)
            .split(frame.area());
        let header_layout = layout[0];

        {
            let keymap = [
                ("hjkl", "Move"),
                ("C-ud", "Page Up/Down"),
                ("Tab", "Focus Other Block"),
                ("f", "Fold All Tree"),
                ("C-l", "Reload"),
                ("C-c", "Quit"),
            ];
            let spans = keymap.into_iter().flat_map(|(key, text)| {
                [
                    Span::styled(format!(" {key} "), Style::new().on_black().white()),
                    Span::styled(format!(" {text}  "), Style::new().gray()),
                ]
            });
            let keymap = Line::from_iter(spans).centered();
            frame.render_widget(keymap, header_layout);
        }

        let layouy = Layout::horizontal([Constraint::Length(50), Constraint::Fill(1)])
            .spacing(2)
            .split(layout[1]);
        let (code_layout, ast_layout) = (layouy[0], layouy[1]);
        {
            let mut block = widgets::Block::bordered().title_top("Code");
            if self.focus == Focus::Code {
                block = block.border_style(Style::new().blue());
            }

            let inner = block.inner(code_layout);
            self.code_block = inner;
            self.code.draw(frame, inner)?;
            frame.render_widget(block, code_layout);
        }
        {
            let mut block = widgets::Block::bordered().title_top("Ast");
            if self.focus == Focus::Ast {
                block = block.border_style(Style::new().blue());
            }

            let inner = block.inner(ast_layout);
            self.ast_block = inner;
            match &mut self.ast {
                Ok(ast) => ast.draw(frame, inner),
                Err(err) => {
                    frame.render_widget(Text::styled(err.as_str(), Style::new().red()), inner);

                    Ok(())
                }
            }?;
            frame.render_widget(block, ast_layout);
        }
        Ok(())
    }

    fn event(&mut self) -> Result<()> {
        match event::read()? {
            Event::Key(key)
                if key.code.is_char('c') && key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                self.exit = true;
            }

            Event::Key(key)
                if key.code.is_tab()
                    && !(self.focus == Focus::Code && self.code.mode() == EditorMode::Insert) =>
            {
                self.focus = match self.focus {
                    Focus::Code => Focus::Ast,
                    Focus::Ast => Focus::Code,
                }
            }

            Event::Key(key)
                if key.code.is_char('l') && key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                let mut cmd = run_script::spawn(
                    &self.cmd,
                    &vec![],
                    &ScriptOptions {
                        input_redirection: IoOptions::Pipe,
                        ..ScriptOptions::new()
                    },
                )?;

                let code = String::from_iter(self.code.code());
                let mut stdin = cmd.stdin.take().unwrap();
                thread::spawn(move || {
                    let _ = stdin.write_all(code.as_bytes());
                    let _ = stdin.flush();
                });
                let output = cmd.wait_with_output()?;

                let output = String::from_utf8_lossy(&output.stdout);

                let mut output = output.lines();
                let sexpr_output = output.next_back();
                let Some(sexpr_output) = sexpr_output else {
                    self.ast = Err("Empty output".to_owned());
                    return Ok(());
                };
                let sexpr = sise::parse_tree(&mut sise::Parser::new(sexpr_output));
                match sexpr {
                    Ok(sexpr) => {
                        let ast = AstComponent::new(sexpr, output.map(|x| x.to_owned()).collect());
                        self.code = CodeComponent::new(mem::take(&mut self.code), ast.pos_lookup());
                        self.ast = Ok(ast);
                    }
                    Err(err) => self.ast = Err(err.to_string()),
                }
            }
            event @ Event::Mouse(mouse) => {
                let pos = Position::new(mouse.column, mouse.row);
                if self.code_block.contains(pos) {
                    self.focus = Focus::Code;
                    self.code_event(event)?;
                } else if self.ast_block.contains(pos) {
                    self.focus = Focus::Ast;
                    self.ast_event(event)?;
                }
            }
            event => match self.focus {
                Focus::Code => {
                    self.code_event(event)?;
                }
                Focus::Ast => {
                    self.ast_event(event)?;
                }
            },
        }
        Ok(())
    }

    fn code_event(&mut self, event: Event) -> Result<()> {
        self.code.on_event(event)?;
        let _ = self.ast.as_mut().ok().map(|ast| {
            ast.select(self.code.selected_tree());
        });
        Ok(())
    }
    fn ast_event(&mut self, event: Event) -> Result<()> {
        self.ast
            .as_mut()
            .ok()
            .map(|ast| {
                ast.on_event(event)?;
                let (s, e) = ast.selected_span();
                self.code.highlight_and_move_cursor(s, e);
                anyhow::Ok(())
            })
            .transpose()?;

        Ok(())
    }
}
