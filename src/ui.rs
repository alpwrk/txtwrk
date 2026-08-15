use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

use crate::app::{App, ConfirmKind, Mode};
use crate::config::Theme;

fn color(name: &str) -> Color {
    match name.to_ascii_lowercase().as_str() {
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        "white" => Color::White,
        "gray" | "grey" => Color::Gray,
        "darkgray" | "darkgrey" => Color::DarkGray,
        "lightred" => Color::LightRed,
        "lightgreen" => Color::LightGreen,
        "lightyellow" => Color::LightYellow,
        "lightblue" => Color::LightBlue,
        "lightmagenta" => Color::LightMagenta,
        "lightcyan" => Color::LightCyan,
        _ => Color::White,
    }
}

pub fn render(app: &mut App, frame: &mut Frame) {
    let theme = app.config.theme.clone();
    let area = frame.area();
    let viewport_height = area.height.saturating_sub(2) as usize;
    app.set_viewport(viewport_height);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);

    match app.mode {
        Mode::Open => render_open(app, frame, chunks[0], &theme),
        _ => render_text(app, frame, chunks[0], &theme),
    }

    render_status(app, frame, chunks[1], &theme);
    render_prompt(app, frame, chunks[2], &theme);
}

fn render_text(app: &mut App, frame: &mut Frame, area: Rect, theme: &Theme) {
    let cursor_line = app.buffer.line_of();
    let viewport = app.viewport_height.max(1);

    if cursor_line < app.scroll {
        app.scroll = cursor_line;
    } else if cursor_line >= app.scroll + viewport {
        app.scroll = cursor_line - viewport + 1;
    }

    let selection = app.buffer.selection_range();
    let find_match = app.buffer.find_match;
    let line_count = app.buffer.line_count();
    let scroll = app.scroll;

    let mut lines: Vec<Line> = Vec::new();
    for line_idx in scroll..(scroll + viewport).min(line_count) {
        let text = app.buffer.line_text(line_idx);
        let line_start = app.buffer.line_start(line_idx);
        let chars: Vec<char> = text.chars().collect();

        let mut spans: Vec<Span> = Vec::new();
        let mut i = 0;
        while i < chars.len() {
            let pos = line_start + i;
            let mut style = Style::default().fg(color(&theme.fg)).bg(color(&theme.bg));

            if let Some((ms, me)) = find_match
                && pos >= ms
                && pos < me
            {
                style = style.fg(color(&theme.match_fg)).bg(color(&theme.match_bg));
            }
            if let Some((ss, se)) = selection
                && pos >= ss
                && pos < se
            {
                style = style
                    .fg(color(&theme.selection_fg))
                    .bg(color(&theme.selection_bg));
            }

            let mut run = String::new();
            let run_style = style;
            while i < chars.len() {
                let pos = line_start + i;
                let mut s = Style::default().fg(color(&theme.fg)).bg(color(&theme.bg));
                if let Some((ms, me)) = find_match
                    && pos >= ms
                    && pos < me
                {
                    s = s.fg(color(&theme.match_fg)).bg(color(&theme.match_bg));
                }
                if let Some((ss, se)) = selection
                    && pos >= ss
                    && pos < se
                {
                    s = s
                        .fg(color(&theme.selection_fg))
                        .bg(color(&theme.selection_bg));
                }
                if s != run_style {
                    break;
                }
                run.push(chars[i]);
                i += 1;
            }
            spans.push(Span::styled(run, run_style));
        }
        lines.push(Line::from(spans));
    }

    while lines.len() < viewport {
        lines.push(Line::from(""));
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, area);

    let cursor_col = app.buffer.cursor - app.buffer.current_line_start();
    let cursor_y = (cursor_line - scroll) as u16;
    let cursor_x = display_col(&app.buffer.line_text(cursor_line), cursor_col) as u16;
    frame.set_cursor_position((area.x + cursor_x, area.y + cursor_y));
}

fn display_col(line: &str, char_col: usize) -> usize {
    let mut width = 0usize;
    for (i, c) in line.chars().enumerate() {
        if i >= char_col {
            break;
        }
        width += unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
    }
    width
}

fn render_status(app: &mut App, frame: &mut Frame, area: Rect, theme: &Theme) {
    let buffer = &app.buffer;
    let mode_name = match app.mode {
        Mode::Normal => "NORMAL",
        Mode::Goto => "GOTO",
        Mode::Find => "FIND",
        Mode::Open => "OPEN",
        Mode::Shell => "SHELL",
        Mode::Confirm => "CONFIRM",
    };
    let insert_mode = match buffer.insert_mode {
        crate::buffer::InsertMode::Insert => "INSERT",
        crate::buffer::InsertMode::Replace => "REPLACE",
    };
    let filename = buffer
        .path
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "[no name]".into());
    let ro = if buffer.read_only { " [read-only]" } else { "" };
    let dirty = if buffer.dirty { " *" } else { "" };
    let line = buffer.line_of() + 1;
    let col = buffer.cursor - buffer.current_line_start() + 1;
    let total = buffer.line_count();

    let left = format!(
        " {} | {} | {} | {}{}{} ",
        mode_name, insert_mode, filename, ro, dirty, ""
    );
    let right = format!(" Ln {}, Col {} / {} ", line, col, total);

    let style = Style::default()
        .fg(color(&theme.status_fg))
        .bg(color(&theme.status_bg))
        .add_modifier(Modifier::BOLD);

    let left_width = left.chars().count() as u16;
    let right_width = right.chars().count() as u16;
    let pad = area.width.saturating_sub(left_width + right_width);
    let mut spans = vec![Span::styled(left, style)];
    if pad > 0 {
        spans.push(Span::styled(" ".repeat(pad as usize), style));
    }
    spans.push(Span::styled(right, style));
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_prompt(app: &mut App, frame: &mut Frame, area: Rect, theme: &Theme) {
    let style = Style::default().fg(color(&theme.fg)).bg(color(&theme.bg));

    let content = match app.mode {
        Mode::Goto | Mode::Find | Mode::Shell => {
            format!("{}{}", app.prompt, app.prompt_input)
        }
        Mode::Confirm => match app.confirm_kind {
            Some(ConfirmKind::DeleteFile) => {
                let target = app
                    .confirm_target
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default();
                format!("Delete {}? [y/N] ", target)
            }
            Some(ConfirmKind::Quit) => "Quit txtwrk? [y/N] ".into(),
            None => String::new(),
        },
        _ => {
            if let Some(msg) = &app.message {
                msg.clone()
            } else {
                String::new()
            }
        }
    };

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(content, style))),
        area,
    );
}

fn render_open(app: &mut App, frame: &mut Frame, area: Rect, theme: &Theme) {
    let items: Vec<ListItem> = app
        .open_entries
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let prefix = if i == app.open_cursor { "> " } else { "  " };
            let name = if e.is_dir {
                format!("{}{}/", prefix, e.name)
            } else {
                format!("{}{}", prefix, e.name)
            };
            let style = if i == app.open_cursor {
                Style::default()
                    .fg(color(&theme.selection_fg))
                    .bg(color(&theme.selection_bg))
            } else {
                Style::default().fg(color(&theme.fg)).bg(color(&theme.bg))
            };
            ListItem::new(Line::from(Span::styled(name, style)))
        })
        .collect();

    let title = format!(" OPEN: {} ", app.open_dir.display());
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD));
    frame.render_widget(list, area);
}
