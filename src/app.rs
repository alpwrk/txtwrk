use std::path::PathBuf;
use std::process::Command;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::buffer::{Buffer, InsertMode};
use crate::config::{Action, Config};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Goto,
    Find,
    Open,
    Shell,
    Confirm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmKind {
    DeleteFile,
    Quit,
}

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,
    pub is_dir: bool,
}

#[derive(Debug, Clone)]
pub struct App {
    pub buffer: Buffer,
    pub mode: Mode,
    pub config: Config,
    pub prompt: String,
    pub prompt_input: String,
    pub message: Option<String>,
    pub scroll: usize,
    pub viewport_height: usize,
    pub open_dir: PathBuf,
    pub open_entries: Vec<FileEntry>,
    pub open_cursor: usize,
    pub confirm_kind: Option<ConfirmKind>,
    pub confirm_target: Option<PathBuf>,
    pub quit_requested: bool,
}

impl App {
    pub fn new(buffer: Buffer, config: Config) -> Self {
        let start_dir = buffer
            .path
            .as_ref()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        let mut app = Self {
            buffer,
            mode: Mode::Normal,
            config,
            prompt: String::new(),
            prompt_input: String::new(),
            message: None,
            scroll: 0,
            viewport_height: 0,
            open_dir: start_dir,
            open_entries: Vec::new(),
            open_cursor: 0,
            confirm_kind: None,
            confirm_target: None,
            quit_requested: false,
        };
        app.refresh_open_dir();
        app
    }

    pub fn set_viewport(&mut self, height: usize) {
        self.viewport_height = height;
    }

    pub fn set_message(&mut self, msg: impl Into<String>) {
        self.message = Some(msg.into());
    }

    pub fn enter_goto(&mut self) {
        self.mode = Mode::Goto;
        self.prompt = "Goto line: ".into();
        self.prompt_input.clear();
    }

    pub fn enter_find(&mut self) {
        self.mode = Mode::Find;
        self.prompt = "Find: ".into();
        self.prompt_input.clear();
    }

    pub fn enter_shell(&mut self) {
        self.mode = Mode::Shell;
        self.prompt = "Shell: ".into();
        self.prompt_input.clear();
    }

    pub fn enter_open(&mut self) {
        self.mode = Mode::Open;
        self.refresh_open_dir();
    }

    pub fn enter_confirm(&mut self, kind: ConfirmKind, target: PathBuf) {
        self.mode = Mode::Confirm;
        self.confirm_kind = Some(kind);
        self.confirm_target = Some(target);
    }

    pub fn exit_confirm(&mut self) {
        self.mode = Mode::Normal;
        self.confirm_kind = None;
        self.confirm_target = None;
    }

    pub fn refresh_open_dir(&mut self) {
        let mut entries = Vec::new();
        if let Ok(read) = std::fs::read_dir(&self.open_dir) {
            for entry in read.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                entries.push(FileEntry { name, is_dir });
            }
        }
        entries.sort_by(|a, b| {
            b.is_dir
                .cmp(&a.is_dir)
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });
        self.open_entries = entries;
        if self.open_cursor >= self.open_entries.len() {
            self.open_cursor = 0;
        }
    }

    pub fn open_selected(&mut self) {
        let Some(entry) = self.open_entries.get(self.open_cursor) else {
            return;
        };
        let path = self.open_dir.join(&entry.name);
        if entry.is_dir {
            self.open_dir = path;
            self.open_cursor = 0;
            self.refresh_open_dir();
        } else {
            match Buffer::from_file(&path) {
                Ok(buf) => {
                    self.buffer = buf;
                    self.mode = Mode::Normal;
                    self.set_message(format!("Opened {}", path.display()));
                }
                Err(e) => self.set_message(format!("Cannot open {}: {}", path.display(), e)),
            }
        }
    }

    pub fn open_parent(&mut self) {
        if self.open_dir.pop() {
            self.open_cursor = 0;
            self.refresh_open_dir();
        }
    }

    pub fn rename_selected(&mut self) {
        let Some(entry) = self.open_entries.get(self.open_cursor) else {
            return;
        };
        self.mode = Mode::Shell;
        self.prompt = format!("Rename {} to: ", entry.name);
        self.prompt_input = entry.name.clone();
    }

    pub fn delete_selected(&mut self) {
        let Some(entry) = self.open_entries.get(self.open_cursor) else {
            return;
        };
        let path = self.open_dir.join(&entry.name);
        self.enter_confirm(ConfirmKind::DeleteFile, path);
    }

    pub fn run_shell_command(&mut self, cmd: &str) {
        let output = Command::new("sh").arg("-c").arg(cmd).output();
        match output {
            Ok(out) => {
                let mut text = String::new();
                if !out.stdout.is_empty() {
                    text.push_str(&String::from_utf8_lossy(&out.stdout));
                }
                if !out.stderr.is_empty() {
                    text.push_str(&String::from_utf8_lossy(&out.stderr));
                }
                if text.is_empty() {
                    text.push_str(&format!(
                        "(no output, exit status {})",
                        out.status.code().unwrap_or(-1)
                    ));
                }
                self.buffer.insert_str(&text);
                self.set_message(format!("Command exited with {}", out.status));
            }
            Err(e) => self.set_message(format!("Failed to run command: {}", e)),
        }
    }

    pub fn handle_key(&mut self, ev: KeyEvent) {
        match self.mode {
            Mode::Normal => self.handle_normal(ev),
            Mode::Goto => self.handle_goto(ev),
            Mode::Find => self.handle_find(ev),
            Mode::Open => self.handle_open(ev),
            Mode::Shell => self.handle_shell(ev),
            Mode::Confirm => self.handle_confirm(ev),
        }
    }

    fn handle_normal(&mut self, ev: KeyEvent) {
        if let Some(action) = self.config.action_for(&ev) {
            self.dispatch_action(action);
            return;
        }
        match ev.code {
            KeyCode::Char(c) => {
                if c == '\n' {
                    self.buffer.insert_char('\n');
                } else {
                    self.buffer.insert_char(c);
                }
            }
            KeyCode::Enter => self.buffer.insert_char('\n'),
            KeyCode::Tab => {
                for _ in 0..self.config.tab_width {
                    self.buffer.insert_char(' ');
                }
            }
            _ => {}
        }
    }

    fn dispatch_action(&mut self, action: Action) {
        match action {
            Action::MoveLeft => self.move_cursor_left(),
            Action::MoveRight => self.move_cursor_right(),
            Action::MoveUp => self.move_cursor_up(),
            Action::MoveDown => self.move_cursor_down(),
            Action::SelectLeft => self.select_extend(-1, 0),
            Action::SelectRight => self.select_extend(1, 0),
            Action::SelectUp => self.select_extend(0, -1),
            Action::SelectDown => self.select_extend(0, 1),
            Action::WordForward => {
                self.buffer.forward_word();
                self.buffer.selection = None;
            }
            Action::WordBackward => {
                self.buffer.backward_word();
                self.buffer.selection = None;
            }
            Action::LineStart => {
                self.buffer.move_cursor(self.buffer.current_line_start());
                self.buffer.selection = None;
            }
            Action::LineEnd => {
                self.buffer.move_cursor(self.buffer.current_line_end());
                self.buffer.selection = None;
            }
            Action::PageUp => {
                self.page_up();
                self.buffer.selection = None;
            }
            Action::PageDown => {
                self.page_down();
                self.buffer.selection = None;
            }
            Action::Goto => self.enter_goto(),
            Action::Backspace => self.buffer.backspace(),
            Action::Delete => self.buffer.delete_forward(),
            Action::InsertToggle => {
                self.buffer.insert_mode = match self.buffer.insert_mode {
                    InsertMode::Insert => InsertMode::Replace,
                    InsertMode::Replace => InsertMode::Insert,
                };
            }
            Action::SelectWord => self.buffer.select_word(),
            Action::SelectLine => self.buffer.select_line(),
            Action::MoveTextLeft => self.buffer.move_selection_word(-1),
            Action::MoveTextRight => self.buffer.move_selection_word(1),
            Action::MoveTextUp => self.buffer.move_selection_line(-1),
            Action::MoveTextDown => self.buffer.move_selection_line(1),
            Action::Find => self.enter_find(),
            Action::Save => self.save(),
            Action::SaveAs => self.save_as(),
            Action::NewFile => self.new_file(),
            Action::Open => self.enter_open(),
            Action::Shell => self.enter_shell(),
            Action::Quit => self.enter_confirm(ConfirmKind::Quit, PathBuf::new()),
            Action::Undo => self.buffer.undo(),
            Action::Redo => self.buffer.redo(),
        }
    }

    fn move_cursor_left(&mut self) {
        if self.buffer.cursor > 0 {
            self.buffer.move_cursor(self.buffer.cursor - 1);
        }
    }

    fn select_extend(&mut self, dx: isize, dy: isize) {
        let anchor = self
            .buffer
            .selection
            .map(|s| s.anchor)
            .unwrap_or(self.buffer.cursor);
        if dx < 0 {
            if self.buffer.cursor > 0 {
                self.buffer.move_cursor(self.buffer.cursor - 1);
            }
        } else if dx > 0 {
            if self.buffer.cursor < self.buffer.len() {
                self.buffer.move_cursor(self.buffer.cursor + 1);
            }
        } else if dy < 0 {
            self.move_cursor_up();
        } else if dy > 0 {
            self.move_cursor_down();
        }
        self.buffer.selection = Some(crate::buffer::Selection::new(anchor, self.buffer.cursor));
    }

    fn move_cursor_right(&mut self) {
        if self.buffer.cursor < self.buffer.len() {
            self.buffer.move_cursor(self.buffer.cursor + 1);
        }
    }

    fn move_cursor_up(&mut self) {
        let line = self.buffer.line_of();
        if line == 0 {
            return;
        }
        let target_col = self.cursor_col();
        let new_line = line - 1;
        let start = self.buffer.line_start(new_line);
        let end = self.buffer.line_end(new_line);
        let pos = (start + target_col).min(end);
        self.buffer.move_cursor(pos);
    }

    fn move_cursor_down(&mut self) {
        let line = self.buffer.line_of();
        if line + 1 >= self.buffer.line_count() {
            return;
        }
        let target_col = self.cursor_col();
        let new_line = line + 1;
        let start = self.buffer.line_start(new_line);
        let end = self.buffer.line_end(new_line);
        let pos = (start + target_col).min(end);
        self.buffer.move_cursor(pos);
    }

    fn cursor_col(&self) -> usize {
        self.buffer.cursor - self.buffer.current_line_start()
    }

    fn page_up(&mut self) {
        let lines = self.viewport_height.max(1);
        let line = self.buffer.line_of();
        let target = line.saturating_sub(lines);
        self.buffer.move_to_line(target);
    }

    fn page_down(&mut self) {
        let lines = self.viewport_height.max(1);
        let line = self.buffer.line_of();
        let target = (line + lines).min(self.buffer.line_count().saturating_sub(1));
        self.buffer.move_to_line(target);
    }

    fn save(&mut self) {
        if self.buffer.read_only {
            self.set_message("Buffer is read-only; use [CA-S] to save as a new file");
            return;
        }
        if self.buffer.path.is_none() {
            self.save_as();
            return;
        }
        match self.buffer.save() {
            Ok(()) => self.set_message("Saved"),
            Err(e) => self.set_message(format!("Save failed: {}", e)),
        }
    }

    fn save_as(&mut self) {
        if self.buffer.read_only {
            self.set_message("Buffer is read-only; cannot save");
            return;
        }
        self.mode = Mode::Shell;
        self.prompt = "Save as: ".into();
        self.prompt_input = self
            .buffer
            .path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
    }

    fn new_file(&mut self) {
        self.buffer = Buffer::new();
        self.mode = Mode::Normal;
        self.set_message("New file");
    }

    fn handle_goto(&mut self, ev: KeyEvent) {
        match ev.code {
            KeyCode::Char('t') | KeyCode::Char('T') => {
                self.buffer.goto_top();
                self.mode = Mode::Normal;
            }
            KeyCode::Char('b') | KeyCode::Char('B') => {
                self.buffer.goto_bottom();
                self.mode = Mode::Normal;
            }
            KeyCode::Char(c) if c.is_ascii_digit() => self.prompt_input.push(c),
            KeyCode::Backspace => {
                self.prompt_input.pop();
            }
            KeyCode::Enter => {
                if let Ok(n) = self.prompt_input.parse::<usize>()
                    && n > 0
                {
                    self.buffer.move_to_line(n - 1);
                }
                self.mode = Mode::Normal;
            }
            KeyCode::Esc => self.mode = Mode::Normal,
            _ => {}
        }
    }

    fn handle_find(&mut self, ev: KeyEvent) {
        if ev.modifiers.contains(KeyModifiers::CONTROL) && ev.code == KeyCode::Char('f') {
            self.enter_find();
            return;
        }
        match ev.code {
            KeyCode::Char(c) => self.prompt_input.push(c),
            KeyCode::Backspace => {
                self.prompt_input.pop();
            }
            KeyCode::Enter => {
                let query = self.prompt_input.clone();
                if self.buffer.find_next(&query) {
                    self.buffer.find_query = Some(query);
                    self.set_message("Found match");
                } else {
                    self.buffer.find_query = Some(query);
                    self.set_message("No match found");
                }
                self.mode = Mode::Normal;
            }
            KeyCode::Esc => {
                self.buffer.find_query = None;
                self.buffer.find_match = None;
                self.mode = Mode::Normal;
            }
            _ => {}
        }
    }

    fn handle_open(&mut self, ev: KeyEvent) {
        if ev.modifiers.contains(KeyModifiers::CONTROL) && ev.code == KeyCode::Char('r') {
            self.rename_selected();
            return;
        }
        match ev.code {
            KeyCode::Up => {
                if self.open_cursor > 0 {
                    self.open_cursor -= 1;
                }
            }
            KeyCode::Down => {
                if self.open_cursor + 1 < self.open_entries.len() {
                    self.open_cursor += 1;
                }
            }
            KeyCode::Enter => self.open_selected(),
            KeyCode::Backspace => self.open_parent(),
            KeyCode::Delete => self.delete_selected(),
            KeyCode::Esc => self.mode = Mode::Normal,
            _ => {}
        }
    }

    fn handle_shell(&mut self, ev: KeyEvent) {
        match ev.code {
            KeyCode::Char(c) => self.prompt_input.push(c),
            KeyCode::Backspace => {
                self.prompt_input.pop();
            }
            KeyCode::Enter => {
                let cmd = self.prompt_input.clone();
                if self.prompt.starts_with("Save as:") {
                    let path = PathBuf::from(cmd.trim());
                    match self.buffer.save_as(&path) {
                        Ok(()) => self.set_message(format!("Saved as {}", path.display())),
                        Err(e) => self.set_message(format!("Save failed: {}", e)),
                    }
                    self.mode = Mode::Normal;
                } else if self.prompt.starts_with("Rename ") {
                    let Some(target) = self.confirm_target.clone().or_else(|| {
                        self.open_entries
                            .get(self.open_cursor)
                            .map(|e| self.open_dir.join(&e.name))
                    }) else {
                        self.mode = Mode::Normal;
                        return;
                    };
                    let new_path = self.open_dir.join(cmd.trim());
                    match std::fs::rename(&target, &new_path) {
                        Ok(()) => self.set_message(format!("Renamed to {}", new_path.display())),
                        Err(e) => self.set_message(format!("Rename failed: {}", e)),
                    }
                    self.mode = Mode::Open;
                    self.refresh_open_dir();
                } else {
                    self.run_shell_command(&cmd);
                    self.mode = Mode::Normal;
                }
            }
            KeyCode::Esc => self.mode = Mode::Normal,
            _ => {}
        }
    }

    fn handle_confirm(&mut self, ev: KeyEvent) {
        match ev.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                match self.confirm_kind {
                    Some(ConfirmKind::DeleteFile) => {
                        if let Some(path) = self.confirm_target.clone() {
                            match std::fs::remove_file(&path) {
                                Ok(()) => self.set_message(format!("Deleted {}", path.display())),
                                Err(e) => self.set_message(format!("Delete failed: {}", e)),
                            }
                        }
                        self.mode = Mode::Open;
                        self.refresh_open_dir();
                    }
                    Some(ConfirmKind::Quit) => {
                        self.quit_requested = true;
                    }
                    None => self.mode = Mode::Normal,
                }
                self.confirm_kind = None;
                self.confirm_target = None;
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.exit_confirm();
            }
            _ => {}
        }
    }
}
