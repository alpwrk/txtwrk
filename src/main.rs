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

    let tutorial = args.iter().any(|a| a == "--tutorial" || a == "-t");
    let file_args: Vec<&String> = args
        .iter()
        .filter(|a| !a.starts_with('-') || a.len() == 1)
        .collect();
    let bad_flags: Vec<&String> = args
        .iter()
        .filter(|a| a.starts_with('-') && a.len() > 1 && *a != "--tutorial" && *a != "-t")
        .collect();
    if !bad_flags.is_empty() {
        eprintln!(
            "txtwrk: unknown option(s): {}",
            bad_flags
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        );
        eprintln!("Usage: txtwrk [file] [--tutorial | -t]");
        std::process::exit(2);
    }

    let buffer = if tutorial {
        Buffer::from_tutorial(TUTORIAL)
    } else if let Some(path) = file_args.first() {
        match Buffer::from_file(&PathBuf::from(path)) {
            Ok(b) => b,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                let mut b = Buffer::new();
                b.path = Some(PathBuf::from(path));
                b
            }
            Err(e) => {
                eprintln!("txtwrk: cannot open {}: {}", path, e);
                std::process::exit(1);
            }
        }
    } else {
        Buffer::new()
    };

    let mut app = App::new(buffer, config);

    struct TerminalGuard;
    impl Drop for TerminalGuard {
        fn drop(&mut self) {
            let _ = disable_raw_mode();
            let _ = execute!(std::io::stdout(), LeaveAlternateScreen);
        }
    }

    enable_raw_mode()?;
    let _guard = TerminalGuard;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut terminal, &mut app);
    let _ = terminal.show_cursor();

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
