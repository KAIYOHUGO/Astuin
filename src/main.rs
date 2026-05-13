use std::{env::args, io::Stdout};

use anyhow::{Result, anyhow};
use astuin::app::App;
use ratatui::{
    crossterm::{
        self, cursor,
        event::{
            DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
        },
        terminal::{EnterAlternateScreen, LeaveAlternateScreen},
    },
    prelude::*,
};

fn main() -> Result<()> {
    let mut app = App::new(
        args()
            .skip(1)
            .next()
            .ok_or_else(|| anyhow!("The ast command is require!"))?,
    );

    let mut term = setup_terminal()?;
    let res = app.run(&mut term);
    teardown_terminal()?;
    res
}
fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    let mut stdout = std::io::stdout();
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(
        stdout,
        EnterAlternateScreen,
        EnableMouseCapture,
        cursor::Hide,
        EnableBracketedPaste
    )?;
    Ok(Terminal::new(CrosstermBackend::new(stdout))?)
}

fn teardown_terminal() -> Result<()> {
    let mut stdout = std::io::stdout();
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(
        stdout,
        LeaveAlternateScreen,
        DisableMouseCapture,
        cursor::Show,
        DisableBracketedPaste
    )?;
    Ok(())
}
