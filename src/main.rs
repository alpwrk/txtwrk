mod app;
mod buffer;
mod config;
mod gap;
mod ui;

use std::io;
use std::path::PathBuf;

use crossterm::event::{self, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::app::App;
use crate::buffer::Buffer;
use crate::config::Config;

const TUTORIAL: &str = include_str!("../assets/tutorial.txt");

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let config = Config::load();

    let buffer = if args.iter().any(|a| a == "--tutorial" || a == "-t") {
        Buffer::from_tutorial(TUTORIAL)
    } else if let Some(path) = args.iter().find(|a| !a.starts_with('-')) {
        match Buffer::from_file(&PathBuf::from(path)) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("txtwrk: cannot open {}: {}", path, e);
                std::process::exit(1);
            }
        }
    } else {
        Buffer::new()
    };

    let mut app = App::new(buffer, config);

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> io::Result<()> {
    loop {
        terminal.draw(|frame| ui::render(app, frame))?;

        if app.quit_requested {
            break;
        }

        if event::poll(std::time::Duration::from_millis(50))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            app.handle_key(key);
        }
    }
    Ok(())
}
