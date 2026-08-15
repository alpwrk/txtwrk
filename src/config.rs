use std::collections::HashMap;
use std::fs;
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
}

#[derive(Debug, Clone)]
pub struct Config {
    pub bindings: HashMap<Action, KeyEvent>,
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
        let mut bindings = HashMap::new();
        bindings.insert(
            Action::MoveLeft,
            key_event(KeyCode::Left, KeyModifiers::NONE),
        );
        bindings.insert(
            Action::MoveRight,
            key_event(KeyCode::Right, KeyModifiers::NONE),
        );
        bindings.insert(Action::MoveUp, key_event(KeyCode::Up, KeyModifiers::NONE));
        bindings.insert(
            Action::MoveDown,
            key_event(KeyCode::Down, KeyModifiers::NONE),
        );
        bindings.insert(
            Action::SelectLeft,
            key_event(KeyCode::Left, KeyModifiers::SHIFT),
        );
        bindings.insert(
            Action::SelectRight,
            key_event(KeyCode::Right, KeyModifiers::SHIFT),
        );
        bindings.insert(
            Action::SelectUp,
            key_event(KeyCode::Up, KeyModifiers::SHIFT),
        );
        bindings.insert(
            Action::SelectDown,
            key_event(KeyCode::Down, KeyModifiers::SHIFT),
        );
        bindings.insert(Action::WordForward, ctrl(KeyCode::Right));
        bindings.insert(Action::WordBackward, ctrl(KeyCode::Left));
        bindings.insert(
            Action::LineStart,
            key_event(KeyCode::Home, KeyModifiers::NONE),
        );
        bindings.insert(Action::LineEnd, key_event(KeyCode::End, KeyModifiers::NONE));
        bindings.insert(
            Action::PageUp,
            key_event(KeyCode::PageUp, KeyModifiers::NONE),
        );
        bindings.insert(
            Action::PageDown,
            key_event(KeyCode::PageDown, KeyModifiers::NONE),
        );
        bindings.insert(Action::Goto, ctrl(KeyCode::Char('g')));
        bindings.insert(
            Action::Backspace,
            key_event(KeyCode::Backspace, KeyModifiers::NONE),
        );
        bindings.insert(
            Action::Delete,
            key_event(KeyCode::Delete, KeyModifiers::NONE),
        );
        bindings.insert(Action::Delete, ctrl(KeyCode::Char('d')));
        bindings.insert(
            Action::InsertToggle,
            key_event(KeyCode::Insert, KeyModifiers::NONE),
        );
        bindings.insert(Action::SelectWord, alt(KeyCode::Char('w')));
        bindings.insert(Action::SelectLine, alt(KeyCode::Char('l')));
        bindings.insert(Action::MoveTextLeft, alt(KeyCode::Left));
        bindings.insert(Action::MoveTextRight, alt(KeyCode::Right));
        bindings.insert(Action::MoveTextUp, alt(KeyCode::Up));
        bindings.insert(Action::MoveTextDown, alt(KeyCode::Down));
        bindings.insert(Action::Find, ctrl(KeyCode::Char('f')));
        bindings.insert(Action::Save, ctrl(KeyCode::Char('s')));
        bindings.insert(Action::SaveAs, ctrl_alt(KeyCode::Char('s')));
        bindings.insert(Action::NewFile, ctrl(KeyCode::Char('n')));
        bindings.insert(Action::Open, ctrl(KeyCode::Char('o')));
        bindings.insert(Action::Shell, ctrl(KeyCode::Char('x')));
        bindings.insert(Action::Quit, ctrl(KeyCode::Char('q')));
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
        let Ok(raw) = fs::read_to_string(&path) else {
            return cfg;
        };
        let Ok(parsed) = toml::from_str::<RawConfig>(&raw) else {
            return cfg;
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
                if let Some(action) = action_from_name(&action_name)
                    && let Some(ev) = parse_key(&key_spec)
                {
                    cfg.bindings.insert(action, ev);
                }
            }
        }
        cfg
    }

    pub fn action_for(&self, ev: &KeyEvent) -> Option<Action> {
        self.bindings
            .iter()
            .find(|(_, bound)| **bound == *ev)
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
        _ => return None,
    })
}

fn parse_key(spec: &str) -> Option<KeyEvent> {
    let parts: Vec<&str> = spec.split('-').collect();
    let mut mods = KeyModifiers::NONE;
    let mut key = "";
    for part in parts {
        match part.to_ascii_lowercase().as_str() {
            "c" | "ctrl" | "control" => mods |= KeyModifiers::CONTROL,
            "a" | "alt" => mods |= KeyModifiers::ALT,
            "s" | "shift" => mods |= KeyModifiers::SHIFT,
            _ => key = part,
        }
    }
    let code = match key.to_ascii_lowercase().as_str() {
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
