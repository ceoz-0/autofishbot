use crossterm::event::{self, Event, KeyCode};
use std::time::Duration;
use anyhow::Result;
use crate::tui::app::App;

pub fn handle_events(app: &mut App) -> Result<()> {
    if event::poll(Duration::from_millis(100))? {
        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => {
                    app.should_quit = true;
                },
                KeyCode::Right | KeyCode::Tab => {
                    app.next_tab();
                },
                KeyCode::Left => {
                    app.previous_tab();
                },
                KeyCode::Char('s') => {
                    app.toggle_bot();
                },
                _ => {}
            }
        }
    }
    Ok(())
}
