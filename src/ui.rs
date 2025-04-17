use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{DefaultTerminal, Frame};

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    color_eyre::install()?;
    let mut terminal = ratatui::init();
    loop {
        terminal.draw(render)?;
        if let Event::Key(key) = event::read()? {
            if handle_key(key) {
                break;
            }
        }
    }
    ratatui::restore();
    Ok(())
}

fn render(frame: &mut Frame) {
    frame.render_widget("hello world", frame.area());
}

fn handle_key(key: KeyEvent) -> bool {
    if key.kind != KeyEventKind::Press {
        return false;
    }
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => true,
        _ => false,
    }
}
