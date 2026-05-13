pub mod app;
pub mod ast;
pub mod code;

use anyhow::Result;
use ratatui::crossterm::event::Event;
use ratatui::prelude::*;
pub trait Component {
    fn on_event(&mut self, event: impl Into<Event>) -> Result<()> {
        let _ = event;
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let _ = (frame, area);
        Ok(())
    }
}
