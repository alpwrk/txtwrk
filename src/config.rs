use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    SelectLeft,
    SelectRight,
    SelectUp,
    SelectDown,
    WordForward,
    WordBackward,
    LineStart,
    LineEnd,
    PageUp,
    PageDown,
    Goto,
    Backspace,
    Delete,
    InsertToggle,
    SelectWord,
    SelectLine,
    MoveTextLeft,
    MoveTextRight,
    MoveTextUp,
    MoveTextDown,
    Find,
    Save,
    SaveAs,
    NewFile,
    Open,
    Shell,
    Quit,
    Undo,
    Redo,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub bindings: HashMap<Action, Vec<KeyEvent>>,
    pub tab_width: u16,
    pub theme: Theme,
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub fg: String,
    pub bg: String,
    pub selection_fg: String,
    pub selection_bg: String,
    pub status_fg: String,
    pub status_bg: String,
    pub match_fg: String,
    pub match_bg: String,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            fg: "White".into(),
            bg: "Black".into(),
            selection_fg: "Black".into(),
            selection_bg: "Cyan".into(),
            status_fg: "Black".into(),
            status_bg: "White".into(),
            match_fg: "Black".into(),
            match_bg: "Yellow".into(),
        }
    }
}

#[derive(Debug, Deserialize, Default)]
struct RawConfig {
    bindings: Option<HashMap<String, String>>,
    tab_width: Option<u16>,
    theme: Option<RawTheme>,
}

#[derive(Debug, Deserialize, Default)]
struct RawTheme {
    fg: Option<String>,
    bg: Option<String>,
    selection_fg: Option<String>,
    selection_bg: Option<String>,
    status_fg: Option<String>,
    status_bg: Option<String>,
    match_fg: Option<String>,
    match_bg: Option<String>,
}

fn key_event(code: KeyCode, mods: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, mods)
}

fn ctrl(code: KeyCode) -> KeyEvent {
    key_event(code, KeyModifiers::CONTROL)
}

fn alt(code: KeyCode) -> KeyEvent {
    key_event(code, KeyModifiers::ALT)
}

fn ctrl_alt(code: KeyCode) -> KeyEvent {
    key_event(code, KeyModifiers::CONTROL | KeyModifiers::ALT)
}

impl Config {
    pub fn defaults() -> Self {
        let mut bindings: HashMap<Action, Vec<KeyEvent>> = HashMap::new();
        let mut bind = |action: Action, ev: KeyEvent| {
            bindings.entry(action).or_default().push(ev);
        };
        bind(
            Action::MoveLeft,
            key_event(KeyCode::Left, KeyModifiers::NONE),
        );
        bind(
            Action::MoveRight,
            key_event(KeyCode::Right, KeyModifiers::NONE),
        );
        bind(Action::MoveUp, key_event(KeyCode::Up, KeyModifiers::NONE));
        bind(
            Action::MoveDown,
            key_event(KeyCode::Down, KeyModifiers::NONE),
        );
        bind(
            Action::SelectLeft,
            key_event(KeyCode::Left, KeyModifiers::SHIFT),
        );
        bind(
            Action::SelectRight,
            key_event(KeyCode::Right, KeyModifiers::SHIFT),
        );
        bind(
            Action::SelectUp,
            key_event(KeyCode::Up, KeyModifiers::SHIFT),
        );
        bind(
            Action::SelectDown,
            key_event(KeyCode::Down, KeyModifiers::SHIFT),
        );
        bind(Action::WordForward, ctrl(KeyCode::Right));
        bind(Action::WordBackward, ctrl(KeyCode::Left));
        bind(
            Action::LineStart,
            key_event(KeyCode::Home, KeyModifiers::NONE),
        );
        bind(Action::LineEnd, key_event(KeyCode::End, KeyModifiers::NONE));
        bind(
            Action::PageUp,
            key_event(KeyCode::PageUp, KeyModifiers::NONE),
        );
        bind(
            Action::PageDown,
            key_event(KeyCode::PageDown, KeyModifiers::NONE),
        );
        bind(Action::Goto, ctrl(KeyCode::Char('g')));
        bind(
            Action::Backspace,
            key_event(KeyCode::Backspace, KeyModifiers::NONE),
        );
        bind(
            Action::Delete,
            key_event(KeyCode::Delete, KeyModifiers::NONE),
        );
        bind(Action::Delete, ctrl(KeyCode::Char('d')));
        bind(
            Action::InsertToggle,
            key_event(KeyCode::Insert, KeyModifiers::NONE),
        );
        bind(Action::SelectWord, alt(KeyCode::Char('w')));
        bind(Action::SelectLine, alt(KeyCode::Char('l')));
        bind(Action::MoveTextLeft, alt(KeyCode::Left));
        bind(Action::MoveTextRight, alt(KeyCode::Right));
        bind(Action::MoveTextUp, alt(KeyCode::Up));
        bind(Action::MoveTextDown, alt(KeyCode::Down));
        bind(Action::Find, ctrl(KeyCode::Char('f')));
        bind(Action::Save, ctrl(KeyCode::Char('s')));
        bind(Action::SaveAs, ctrl_alt(KeyCode::Char('s')));
        bind(Action::NewFile, ctrl(KeyCode::Char('n')));
        bind(Action::Open, ctrl(KeyCode::Char('o')));
        bind(Action::Shell, ctrl(KeyCode::Char('x')));
        bind(Action::Quit, ctrl(KeyCode::Char('q')));
        bind(Action::Undo, ctrl(KeyCode::Char('z')));
        bind(Action::Redo, ctrl(KeyCode::Char('y')));
        Self {
            bindings,
            tab_width: 4,
            theme: Theme::default(),
        }
    }

    pub fn load() -> Self {
        let mut cfg = Self::defaults();
        let Some(dir) = dirs::config_dir() else {
            return cfg;
        };
        let path: PathBuf = dir.join("txtwrk").join("config.toml");
        let raw = match fs::read_to_string(&path) {
            Ok(raw) => raw,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return cfg,
            Err(e) => {
                eprintln!("txtwrk: cannot read config {}: {}", path.display(), e);
                return cfg;
            }
        };
        let parsed = match toml::from_str::<RawConfig>(&raw) {
            Ok(parsed) => parsed,
            Err(e) => {
                eprintln!(
                    "txtwrk: ignoring malformed config {}: {}",
                    path.display(),
                    e
                );
                return cfg;
            }
        };
        if let Some(tw) = parsed.tab_width {
            cfg.tab_width = tw;
        }
        if let Some(theme) = parsed.theme {
            let t = &mut cfg.theme;
            if let Some(v) = theme.fg {
                t.fg = v;
            }
            if let Some(v) = theme.bg {
                t.bg = v;
            }
            if let Some(v) = theme.selection_fg {
                t.selection_fg = v;
            }
            if let Some(v) = theme.selection_bg {
                t.selection_bg = v;
            }
            if let Some(v) = theme.status_fg {
                t.status_fg = v;
            }
            if let Some(v) = theme.status_bg {
                t.status_bg = v;
            }
            if let Some(v) = theme.match_fg {
                t.match_fg = v;
            }
            if let Some(v) = theme.match_bg {
                t.match_bg = v;
            }
        }
        if let Some(bindings) = parsed.bindings {
            for (action_name, key_spec) in bindings {
                let Some(action) = action_from_name(&action_name) else {
                    eprintln!("txtwrk: ignoring unknown binding \"{}\"", action_name);
                    continue;
                };
                match parse_key(&key_spec) {
                    Some(ev) => {
                        cfg.bindings.entry(action).or_default().push(ev);
                    }
                    None => {
                        eprintln!(
                            "txtwrk: ignoring unparseable binding {} = \"{}\"",
                            action_name, key_spec
                        );
                    }
                }
            }
        }
        cfg
    }

    pub fn action_for(&self, ev: &KeyEvent) -> Option<Action> {
        self.bindings
            .iter()
            .find(|(_, keys)| keys.iter().any(|k| k == ev))
            .map(|(action, _)| *action)
    }
}

fn action_from_name(name: &str) -> Option<Action> {
    Some(match name {
        "move_left" => Action::MoveLeft,
        "move_right" => Action::MoveRight,
        "move_up" => Action::MoveUp,
        "move_down" => Action::MoveDown,
        "select_left" => Action::SelectLeft,
        "select_right" => Action::SelectRight,
        "select_up" => Action::SelectUp,
        "select_down" => Action::SelectDown,
        "word_forward" => Action::WordForward,
        "word_backward" => Action::WordBackward,
        "line_start" => Action::LineStart,
        "line_end" => Action::LineEnd,
        "page_up" => Action::PageUp,
        "page_down" => Action::PageDown,
        "goto" => Action::Goto,
        "backspace" => Action::Backspace,
        "delete" => Action::Delete,
        "insert_toggle" => Action::InsertToggle,
        "select_word" => Action::SelectWord,
        "select_line" => Action::SelectLine,
        "move_text_left" => Action::MoveTextLeft,
        "move_text_right" => Action::MoveTextRight,
        "move_text_up" => Action::MoveTextUp,
        "move_text_down" => Action::MoveTextDown,
        "find" => Action::Find,
        "save" => Action::Save,
        "save_as" => Action::SaveAs,
        "new_file" => Action::NewFile,
        "open" => Action::Open,
        "shell" => Action::Shell,
        "quit" => Action::Quit,
        "undo" => Action::Undo,
        "redo" => Action::Redo,
        _ => return None,
    })
}

fn parse_key(spec: &str) -> Option<KeyEvent> {
    let parts: Vec<&str> = spec.split('-').collect();
    if parts.is_empty() {
        return None;
    }
    let (mod_parts, key_part) = parts.split_at(parts.len() - 1);
    let key = key_part[0].to_ascii_lowercase();
    let mut mods = KeyModifiers::NONE;
    for part in mod_parts {
        if part.is_empty() {
            return None;
        }
        match part.to_ascii_lowercase().as_str() {
            "c" | "ctrl" | "control" => mods |= KeyModifiers::CONTROL,
            "a" | "alt" => mods |= KeyModifiers::ALT,
            "s" | "shift" => mods |= KeyModifiers::SHIFT,
            _ => {
                for c in part.to_ascii_lowercase().chars() {
                    match c {
                        'c' => mods |= KeyModifiers::CONTROL,
                        'a' => mods |= KeyModifiers::ALT,
                        's' => mods |= KeyModifiers::SHIFT,
                        _ => return None,
                    }
                }
            }
        }
    }
    let code = match key.as_str() {
        "enter" => KeyCode::Enter,
        "esc" | "escape" => KeyCode::Esc,
        "tab" => KeyCode::Tab,
        "backspace" => KeyCode::Backspace,
        "delete" | "del" => KeyCode::Delete,
        "insert" => KeyCode::Insert,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "pageup" | "pgup" => KeyCode::PageUp,
        "pagedown" | "pgdn" => KeyCode::PageDown,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "space" => KeyCode::Char(' '),
        "f1" => KeyCode::F(1),
        "f2" => KeyCode::F(2),
        "f3" => KeyCode::F(3),
        "f4" => KeyCode::F(4),
        "f5" => KeyCode::F(5),
        "f6" => KeyCode::F(6),
        "f7" => KeyCode::F(7),
        "f8" => KeyCode::F(8),
        "f9" => KeyCode::F(9),
        "f10" => KeyCode::F(10),
        "f11" => KeyCode::F(11),
        "f12" => KeyCode::F(12),
        _ => {
            let mut chars = key.chars();
            let c = chars.next()?;
            if chars.next().is_some() {
                return None;
            }
            KeyCode::Char(c)
        }
    };
    Some(KeyEvent::new(code, mods))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_spec(spec: &str, code: KeyCode, mods: KeyModifiers) {
        assert_eq!(
            parse_key(spec),
            Some(KeyEvent::new(code, mods)),
            "spec: {}",
            spec
        );
    }

    #[test]
    fn parse_key_simple() {
        assert_spec("left", KeyCode::Left, KeyModifiers::NONE);
        assert_spec("c-s", KeyCode::Char('s'), KeyModifiers::CONTROL);
        assert_spec("a-s", KeyCode::Char('s'), KeyModifiers::ALT);
        assert_spec("s-s", KeyCode::Char('s'), KeyModifiers::SHIFT);
        assert_spec("s-left", KeyCode::Left, KeyModifiers::SHIFT);
        assert_spec(
            "ca-s",
            KeyCode::Char('s'),
            KeyModifiers::CONTROL | KeyModifiers::ALT,
        );
        assert_spec(
            "c-a-f1",
            KeyCode::F(1),
            KeyModifiers::CONTROL | KeyModifiers::ALT,
        );
        assert_spec(
            "ctrl-alt-delete",
            KeyCode::Delete,
            KeyModifiers::CONTROL | KeyModifiers::ALT,
        );
        assert_spec("space", KeyCode::Char(' '), KeyModifiers::NONE);
        assert_spec("enter", KeyCode::Enter, KeyModifiers::NONE);
    }

    #[test]
    fn parse_key_invalid() {
        assert_eq!(parse_key(""), None);
        assert_eq!(parse_key("-"), None);
        assert_eq!(parse_key("c-"), None);
        assert_eq!(parse_key("-s"), None);
        assert_eq!(parse_key("x-left"), None);
        assert_eq!(parse_key("multi-char"), None);
    }

    #[test]
    fn action_names() {
        assert_eq!(action_from_name("save"), Some(Action::Save));
        assert_eq!(action_from_name("save_as"), Some(Action::SaveAs));
        assert_eq!(action_from_name("quit"), Some(Action::Quit));
        assert_eq!(action_from_name("nonsense"), None);
    }

    #[test]
    fn defaults_bind_delete_twice() {
        let cfg = Config::defaults();
        let delete = cfg
            .action_for(&key_event(KeyCode::Delete, KeyModifiers::NONE))
            .unwrap();
        let ctrl_d = cfg
            .action_for(&key_event(KeyCode::Char('d'), KeyModifiers::CONTROL))
            .unwrap();
        assert_eq!(delete, Action::Delete);
        assert_eq!(ctrl_d, Action::Delete);
    }
}
