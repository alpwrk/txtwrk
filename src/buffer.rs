use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::gap::GapBuffer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertMode {
    Insert,
    Replace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    pub anchor: usize,
    pub cursor: usize,
}

impl Selection {
    pub fn new(anchor: usize, cursor: usize) -> Self {
        Self { anchor, cursor }
    }

    pub fn start(&self) -> usize {
        self.anchor.min(self.cursor)
    }

    pub fn end(&self) -> usize {
        self.anchor.max(self.cursor)
    }

    pub fn range(&self) -> std::ops::Range<usize> {
        self.start()..self.end()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditOp {
    Insert {
        pos: usize,
        text: String,
    },
    Delete {
        pos: usize,
        text: String,
        forward: bool,
    },
    Replace {
        pos: usize,
        old: String,
        new: String,
    },
    ReplaceAll {
        old: String,
        new: String,
    },
}

#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub op: EditOp,
    pub cursor_before: usize,
    pub cursor_after: usize,
    pub selection_before: Option<Selection>,
    pub selection_after: Option<Selection>,
    pub dirty_before: bool,
    pub dirty_after: bool,
}

#[derive(Debug, Clone)]
pub struct Buffer {
    pub gap: GapBuffer,
    pub cursor: usize,
    pub selection: Option<Selection>,
    pub insert_mode: InsertMode,
    pub path: Option<PathBuf>,
    pub read_only: bool,
    pub dirty: bool,
    pub find_query: Option<String>,
    pub find_match: Option<(usize, usize)>,
    pub undo_stack: Vec<HistoryEntry>,
    pub redo_stack: Vec<HistoryEntry>,
}

impl Buffer {
    pub fn new() -> Self {
        Self {
            gap: GapBuffer::new(),
            cursor: 0,
            selection: None,
            insert_mode: InsertMode::Insert,
            path: None,
            read_only: false,
            dirty: false,
            find_query: None,
            find_match: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn from_file(path: &Path) -> io::Result<Self> {
        let content = fs::read_to_string(path)?;
        let mut b = Self::new();
        b.gap = GapBuffer::from_str(&content);
        b.path = Some(path.to_path_buf());
        Ok(b)
    }

    pub fn from_tutorial(content: &str) -> Self {
        let mut b = Self::new();
        b.gap = GapBuffer::from_str(content);
        b.read_only = true;
        b
    }

    pub fn len(&self) -> usize {
        self.gap.len()
    }

    pub fn text(&self) -> String {
        self.gap.to_string()
    }

    pub fn line_count(&self) -> usize {
        self.gap.line_count()
    }

    pub fn line_text(&self, line: usize) -> String {
        self.gap.line_text(line)
    }

    pub fn line_of(&self) -> usize {
        self.gap.line_of(self.cursor)
    }

    pub fn line_start(&self, line: usize) -> usize {
        self.gap.line_start(line)
    }

    pub fn line_end(&self, line: usize) -> usize {
        self.gap.line_end(line)
    }

    pub fn current_line_start(&self) -> usize {
        self.gap.line_start(self.line_of())
    }

    pub fn current_line_end(&self) -> usize {
        self.gap.line_end(self.line_of())
    }

    pub fn move_cursor(&mut self, pos: usize) {
        self.cursor = pos.min(self.len());
    }

    fn invalidate_find(&mut self) {
        self.find_match = None;
    }

    pub fn selection_range(&self) -> Option<(usize, usize)> {
        self.selection.map(|s| (s.start(), s.end()))
    }

    pub fn insert_char(&mut self, c: char) {
        if self.read_only {
            return;
        }
        self.invalidate_find();
        let cursor_before = self.cursor;
        let selection_before = self.selection;
        let replaced = self.selection.map(|sel| {
            let text = self.text_range(sel.start(), sel.end());
            self.gap.delete_range(sel.range());
            self.cursor = sel.start();
            self.selection = None;
            (sel.start(), text)
        });
        match self.insert_mode {
            InsertMode::Insert => {
                self.gap.insert(self.cursor, c);
                self.cursor += 1;
                let op = match replaced {
                    Some((pos, old)) => EditOp::Replace {
                        pos,
                        old,
                        new: c.to_string(),
                    },
                    None => EditOp::Insert {
                        pos: self.cursor - 1,
                        text: c.to_string(),
                    },
                };
                self.record(op, cursor_before, selection_before);
            }
            InsertMode::Replace => {
                if self.cursor < self.len() {
                    let old = self.text_range(self.cursor, self.cursor + 1);
                    self.gap
                        .replace_range(self.cursor..self.cursor + 1, &c.to_string());
                    self.cursor += 1;
                    let op = match replaced {
                        Some((pos, sel_text)) => EditOp::Replace {
                            pos,
                            old: format!("{}{}", sel_text, old),
                            new: c.to_string(),
                        },
                        None => EditOp::Replace {
                            pos: self.cursor - 1,
                            old,
                            new: c.to_string(),
                        },
                    };
                    self.record(op, cursor_before, selection_before);
                } else {
                    self.gap.insert(self.cursor, c);
                    self.cursor += 1;
                    let op = match replaced {
                        Some((pos, old)) => EditOp::Replace {
                            pos,
                            old,
                            new: c.to_string(),
                        },
                        None => EditOp::Insert {
                            pos: self.cursor - 1,
                            text: c.to_string(),
                        },
                    };
                    self.record(op, cursor_before, selection_before);
                }
            }
        }
        self.dirty = true;
    }

    pub fn insert_pair(&mut self, open: char, close: char) {
        if self.read_only {
            return;
        }
        if let Some(sel) = self.selection {
            let start = sel.start();
            let end = sel.end();
            let old = self.text_range(start, end);
            let new = format!("{}{}{}", open, old, close);
            self.gap.delete_range(sel.range());
            self.cursor = start;
            self.selection = None;
            self.gap.insert_str(start, &new);
            self.cursor = start + old.chars().count() + 1;
            self.record(
                EditOp::Replace {
                    pos: start,
                    old,
                    new,
                },
                end,
                Some(sel),
            );
            self.dirty = true;
            return;
        }
        let mut s = String::with_capacity(2);
        s.push(open);
        s.push(close);
        if self.insert_mode == InsertMode::Replace && self.cursor < self.len() {
            let cursor_before = self.cursor;
            let selection_before = self.selection;
            let old = self.text_range(self.cursor, self.cursor + 1);
            self.gap.replace_range(self.cursor..self.cursor + 1, &s);
            self.cursor += 1;
            self.record(
                EditOp::Replace {
                    pos: self.cursor - 1,
                    old,
                    new: s,
                },
                cursor_before,
                selection_before,
            );
            self.dirty = true;
            return;
        }
        self.insert_str(&s);
        self.cursor -= 1;
        if let Some(prev) = self.undo_stack.last_mut() {
            prev.cursor_after = self.cursor;
            prev.selection_after = self.selection;
        }
    }

    pub fn should_pair_quote(&self) -> bool {
        if self.cursor == 0 {
            return true;
        }
        self.gap
            .char_at(self.cursor - 1)
            .is_some_and(|c| c.is_whitespace() || matches!(c, '(' | '[' | '{'))
    }

    pub fn insert_str(&mut self, s: &str) {
        if self.read_only {
            return;
        }
        if s.is_empty() {
            return;
        }
        self.invalidate_find();
        let cursor_before = self.cursor;
        let selection_before = self.selection;
        let replaced = self.selection.map(|sel| {
            let text = self.text_range(sel.start(), sel.end());
            self.gap.delete_range(sel.range());
            self.cursor = sel.start();
            self.selection = None;
            (sel.start(), text)
        });
        self.gap.insert_str(self.cursor, s);
        self.cursor += s.chars().count();
        let op = match replaced {
            Some((pos, old)) => EditOp::Replace {
                pos,
                old,
                new: s.to_string(),
            },
            None => EditOp::Insert {
                pos: self.cursor - s.chars().count(),
                text: s.to_string(),
            },
        };
        self.record(op, cursor_before, selection_before);
        self.dirty = true;
    }

    pub fn delete_selection(&mut self) {
        if self.read_only {
            return;
        }
        self.invalidate_find();
        if let Some(sel) = self.selection {
            let cursor_before = self.cursor;
            let selection_before = self.selection;
            let text = self.text_range(sel.start(), sel.end());
            self.gap.delete_range(sel.range());
            self.cursor = sel.start();
            self.selection = None;
            self.record(
                EditOp::Delete {
                    pos: sel.start(),
                    text,
                    forward: true,
                },
                cursor_before,
                selection_before,
            );
            self.dirty = true;
        }
    }

    pub fn backspace(&mut self) {
        if self.read_only {
            return;
        }
        self.invalidate_find();
        if let Some(_sel) = self.selection {
            self.delete_selection();
            return;
        }
        if self.cursor > 0 {
            let cursor_before = self.cursor;
            let selection_before = self.selection;
            let text = self.text_range(self.cursor - 1, self.cursor);
            self.gap.delete_before(self.cursor);
            self.cursor -= 1;
            self.record(
                EditOp::Delete {
                    pos: self.cursor,
                    text,
                    forward: false,
                },
                cursor_before,
                selection_before,
            );
            self.dirty = true;
        }
    }

    pub fn delete_forward(&mut self) {
        if self.read_only {
            return;
        }
        self.invalidate_find();
        if let Some(_sel) = self.selection {
            self.delete_selection();
            return;
        }
        if self.cursor < self.len() {
            let cursor_before = self.cursor;
            let selection_before = self.selection;
            let text = self.text_range(self.cursor, self.cursor + 1);
            self.gap.delete_after(self.cursor);
            self.record(
                EditOp::Delete {
                    pos: self.cursor,
                    text,
                    forward: true,
                },
                cursor_before,
                selection_before,
            );
            self.dirty = true;
        }
    }

    pub fn text_range(&self, start: usize, end: usize) -> String {
        let mut s = String::new();
        for i in start..end.min(self.len()) {
            if let Some(c) = self.gap.char_at(i) {
                s.push(c);
            }
        }
        s
    }

    pub fn char_at(&self, pos: usize) -> Option<char> {
        self.gap.char_at(pos)
    }

    pub fn move_selection_word(&mut self, dir: isize) {
        if self.read_only {
            return;
        }
        self.invalidate_find();
        let Some(sel) = self.selection else {
            return;
        };
        let start = sel.start();
        let end = sel.end();
        let len = self.len();
        let is_ws = |c: char| c.is_whitespace();
        let cursor_before = self.cursor;
        let (old, new, sel_start, sel_end) = if dir > 0 {
            let mut i = end;
            while i < len && self.gap.char_at(i).is_some_and(is_ws) {
                i += 1;
            }
            let word_start = i;
            while i < len && self.gap.char_at(i).is_some_and(|c| !is_ws(c)) {
                i += 1;
            }
            let word_end = i;
            if word_start == word_end {
                return;
            }
            let old = self.text();
            let sel_text = self.text_range(start, end);
            let following = self.text_range(word_start, word_end);
            let ws = self.text_range(end, word_start);
            let prefix = self.text_range(0, start);
            let suffix = self.text_range(word_end, len);
            let new = format!("{}{}{}{}", prefix, following, ws, sel_text);
            let new_full = format!("{}{}", new, suffix);
            let sel_start = start + following.chars().count() + ws.chars().count();
            let sel_end = sel_start + sel_text.chars().count();
            (old, new_full, sel_start, sel_end)
        } else {
            let mut i = start;
            while i > 0 && self.gap.char_at(i - 1).is_some_and(is_ws) {
                i -= 1;
            }
            let word_end = i;
            while i > 0 && self.gap.char_at(i - 1).is_some_and(|c| !is_ws(c)) {
                i -= 1;
            }
            let word_start = i;
            if word_start == word_end {
                return;
            }
            let old = self.text();
            let sel_text = self.text_range(start, end);
            let prev = self.text_range(word_start, word_end);
            let ws = self.text_range(word_end, start);
            let prefix = self.text_range(0, word_start);
            let suffix = self.text_range(end, len);
            let new = format!("{}{}{}{}", prefix, sel_text, ws, prev);
            let new_full = format!("{}{}", new, suffix);
            let sel_start = word_start;
            let sel_end = sel_start + sel_text.chars().count();
            (old, new_full, sel_start, sel_end)
        };
        self.gap = GapBuffer::from_str(&new);
        self.cursor = sel_end;
        self.selection = Some(Selection::new(sel_start, sel_end));
        self.record(EditOp::ReplaceAll { old, new }, cursor_before, Some(sel));
        self.dirty = true;
    }

    pub fn move_selection_line(&mut self, dir: isize) {
        if self.read_only {
            return;
        }
        self.invalidate_find();
        let Some(sel) = self.selection else {
            return;
        };
        let start = sel.start();
        let end = sel.end();
        let len = self.len();
        let cursor_before = self.cursor;
        let (old, new, sel_start, sel_end) = if dir > 0 {
            let line = self.gap.line_of(end);
            let next_line = line + 1;
            if next_line >= self.gap.line_count() {
                return;
            }
            let next_start = self.gap.line_start(next_line);
            let next_end = self.gap.line_end(next_line);
            let next_end_incl = if next_end < len && self.gap.char_at(next_end) == Some('\n') {
                next_end + 1
            } else {
                next_end
            };
            let old = self.text();
            let sel_text = self.text_range(start, end);
            let following = self.text_range(next_start, next_end);
            let ws = self.text_range(end, next_start);
            let prefix = self.text_range(0, start);
            let suffix = self.text_range(next_end_incl, len);
            let new = format!("{}{}{}{}", prefix, following, ws, sel_text);
            let new_full = format!("{}{}", new, suffix);
            let sel_start = start + following.chars().count() + ws.chars().count();
            let sel_end = sel_start + sel_text.chars().count();
            (old, new_full, sel_start, sel_end)
        } else {
            let line = self.gap.line_of(start);
            if line == 0 {
                return;
            }
            let prev_start = self.gap.line_start(line - 1);
            let prev_end = self.gap.line_end(line - 1);
            let old = self.text();
            let sel_text = self.text_range(start, end);
            let prev = self.text_range(prev_start, prev_end);
            let ws = self.text_range(prev_end, start);
            let prefix = self.text_range(0, prev_start);
            let suffix = self.text_range(end, len);
            let new = format!("{}{}{}{}", prefix, sel_text, ws, prev);
            let new_full = format!("{}{}", new, suffix);
            let sel_start = prev_start;
            let sel_end = sel_start + sel_text.chars().count();
            (old, new_full, sel_start, sel_end)
        };
        self.gap = GapBuffer::from_str(&new);
        self.cursor = sel_end;
        self.selection = Some(Selection::new(sel_start, sel_end));
        self.record(EditOp::ReplaceAll { old, new }, cursor_before, Some(sel));
        self.dirty = true;
    }

    pub fn select_word(&mut self) {
        let (start, end) = self.word_bounds(self.cursor);
        self.cursor = end;
        self.selection = Some(Selection::new(start, end));
    }

    pub fn select_line(&mut self) {
        let line = self.line_of();
        let start = self.line_start(line);
        let end = self.line_end(line);
        self.cursor = end;
        self.selection = Some(Selection::new(start, end));
    }

    pub fn word_bounds(&self, pos: usize) -> (usize, usize) {
        let is_word = |c: char| c.is_alphanumeric() || c == '_';
        let mut start = pos;
        while start > 0 {
            let prev = self.gap.char_at(start - 1).unwrap_or(' ');
            if is_word(prev) {
                start -= 1;
            } else {
                break;
            }
        }
        let mut end = pos;
        while let Some(c) = self.gap.char_at(end) {
            if is_word(c) {
                end += 1;
            } else {
                break;
            }
        }
        (start, end)
    }

    pub fn forward_word(&mut self) {
        let is_word = |c: char| c.is_alphanumeric() || c == '_';
        let mut pos = self.cursor;
        while let Some(c) = self.gap.char_at(pos) {
            if is_word(c) {
                pos += 1;
            } else {
                break;
            }
        }
        while let Some(c) = self.gap.char_at(pos) {
            if c.is_whitespace() {
                pos += 1;
            } else {
                break;
            }
        }
        self.move_cursor(pos);
    }

    pub fn backward_word(&mut self) {
        let is_word = |c: char| c.is_alphanumeric() || c == '_';
        let mut pos = self.cursor;
        while pos > 0 {
            let prev = self.gap.char_at(pos - 1).unwrap_or(' ');
            if prev.is_whitespace() {
                pos -= 1;
            } else {
                break;
            }
        }
        while pos > 0 {
            let prev = self.gap.char_at(pos - 1).unwrap_or(' ');
            if is_word(prev) {
                pos -= 1;
            } else {
                break;
            }
        }
        self.move_cursor(pos);
    }

    pub fn move_to_line(&mut self, line: usize) {
        let line = line.min(self.line_count().saturating_sub(1));
        self.move_cursor(self.line_start(line));
    }

    pub fn goto_top(&mut self) {
        self.move_cursor(0);
    }

    pub fn goto_bottom(&mut self) {
        self.move_cursor(self.len());
    }

    pub fn find_next(&mut self, query: &str) -> bool {
        if query.is_empty() {
            self.find_match = None;
            return false;
        }
        let text = self.text();
        let query_chars = query.chars().count();
        let mut search_from = self.cursor;
        if let Some((ms, me)) = self.find_match
            && ms == self.cursor
            && me > ms
        {
            search_from = ms + 1;
        }
        let mut byte_from = text
            .char_indices()
            .nth(search_from)
            .map(|(b, _)| b)
            .unwrap_or(text.len());
        let mut wrapped = false;
        loop {
            if let Some(rel) = text[byte_from..].find(query) {
                let abs_byte = byte_from + rel;
                let abs = text[..abs_byte].chars().count();
                if wrapped && abs >= self.cursor {
                    break;
                }
                self.cursor = abs;
                self.find_match = Some((abs, abs + query_chars));
                return true;
            }
            if wrapped {
                break;
            }
            byte_from = 0;
            wrapped = true;
        }
        self.find_match = None;
        false
    }

    fn write_atomic(&self, path: &Path) -> io::Result<()> {
        let target = if let Ok(meta) = fs::symlink_metadata(path) {
            if meta.file_type().is_symlink() {
                match fs::canonicalize(path) {
                    Ok(real) => real,
                    Err(_) => return fs::write(path, self.text()),
                }
            } else {
                path.to_path_buf()
            }
        } else {
            path.to_path_buf()
        };
        let dir = target.parent().filter(|d| !d.as_os_str().is_empty());
        let file_name = target
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "buffer".into());
        let tmp = match dir {
            Some(d) => d.join(format!(".{}.txtwrk.tmp", file_name)),
            None => PathBuf::from(format!(".{}.txtwrk.tmp", file_name)),
        };
        let result = (|| {
            fs::write(&tmp, self.text())?;
            if let Ok(meta) = fs::metadata(&target) {
                fs::set_permissions(&tmp, meta.permissions())?;
            }
            fs::rename(&tmp, &target)
        })();
        if result.is_err() {
            let _ = fs::remove_file(&tmp);
        }
        result
    }

    pub fn save(&mut self) -> io::Result<()> {
        if self.read_only {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "buffer is read-only",
            ));
        }
        let path = self
            .path
            .clone()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no filename set"))?;
        self.write_atomic(&path)?;
        self.dirty = false;
        Ok(())
    }

    pub fn save_as(&mut self, path: &Path) -> io::Result<()> {
        if self.read_only {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "buffer is read-only",
            ));
        }
        self.write_atomic(path)?;
        self.path = Some(path.to_path_buf());
        self.dirty = false;
        Ok(())
    }

    fn record(&mut self, op: EditOp, cursor_before: usize, selection_before: Option<Selection>) {
        let cursor_after = self.cursor;
        let selection_after = self.selection;
        let dirty_before = self.dirty;
        let dirty_after = true;

        let coalesce = match (self.undo_stack.last(), &op) {
            (Some(prev), EditOp::Insert { pos, text })
                if matches!(prev.op, EditOp::Insert { .. }) =>
            {
                if let EditOp::Insert {
                    pos: prev_pos,
                    text: prev_text,
                } = &prev.op
                {
                    *pos == *prev_pos + prev_text.chars().count()
                        && cursor_before == prev.cursor_after
                        && selection_before == prev.selection_after
                } else {
                    false
                }
            }
            (Some(prev), EditOp::Delete { pos, text, forward })
                if matches!(prev.op, EditOp::Delete { .. }) =>
            {
                if let EditOp::Delete {
                    pos: prev_pos,
                    text: _prev_text,
                    forward: prev_forward,
                } = &prev.op
                {
                    let pos_ok = if *forward {
                        *pos == *prev_pos
                    } else {
                        *pos + text.chars().count() == *prev_pos
                    };
                    pos_ok
                        && *forward == *prev_forward
                        && cursor_before == prev.cursor_after
                        && selection_before == prev.selection_after
                } else {
                    false
                }
            }
            _ => false,
        };

        if coalesce {
            if let Some(prev) = self.undo_stack.last_mut() {
                match (&mut prev.op, &op) {
                    (EditOp::Insert { text, .. }, EditOp::Insert { text: new_text, .. }) => {
                        text.push_str(new_text);
                    }
                    (
                        EditOp::Delete {
                            text,
                            forward: true,
                            ..
                        },
                        EditOp::Delete {
                            text: new_text,
                            forward: true,
                            ..
                        },
                    ) => {
                        text.push_str(new_text);
                    }
                    (
                        EditOp::Delete {
                            pos,
                            text,
                            forward: false,
                            ..
                        },
                        EditOp::Delete {
                            pos: new_pos,
                            text: new_text,
                            forward: false,
                            ..
                        },
                    ) => {
                        text.insert_str(0, new_text);
                        *pos = *new_pos;
                    }
                    _ => {}
                }
                prev.cursor_after = cursor_after;
                prev.selection_after = selection_after;
                prev.dirty_after = dirty_after;
            }
        } else {
            self.undo_stack.push(HistoryEntry {
                op,
                cursor_before,
                cursor_after,
                selection_before,
                selection_after,
                dirty_before,
                dirty_after,
            });
        }
        self.redo_stack.clear();
    }

    pub fn undo(&mut self) {
        if self.read_only {
            return;
        }
        self.invalidate_find();
        let Some(entry) = self.undo_stack.pop() else {
            return;
        };
        match &entry.op {
            EditOp::Insert { pos, text } => {
                self.gap.delete_range(*pos..pos + text.chars().count());
            }
            EditOp::Delete { pos, text, .. } => {
                self.gap.insert_str(*pos, text);
            }
            EditOp::Replace { pos, old, new } => {
                self.gap.replace_range(*pos..pos + new.chars().count(), old);
            }
            EditOp::ReplaceAll { old, .. } => {
                self.gap = GapBuffer::from_str(old);
            }
        }
        self.cursor = entry.cursor_before;
        self.selection = entry.selection_before;
        self.dirty = entry.dirty_before;
        self.redo_stack.push(entry);
    }

    pub fn redo(&mut self) {
        if self.read_only {
            return;
        }
        self.invalidate_find();
        let Some(entry) = self.redo_stack.pop() else {
            return;
        };
        match &entry.op {
            EditOp::Insert { pos, text } => {
                self.gap.insert_str(*pos, text);
            }
            EditOp::Delete { pos, text, .. } => {
                self.gap.delete_range(*pos..pos + text.chars().count());
            }
            EditOp::Replace { pos, old, new } => {
                self.gap.replace_range(*pos..pos + old.chars().count(), new);
            }
            EditOp::ReplaceAll { new, .. } => {
                self.gap = GapBuffer::from_str(new);
            }
        }
        self.cursor = entry.cursor_after;
        self.selection = entry.selection_after;
        self.dirty = entry.dirty_after;
        self.undo_stack.push(entry);
    }
}

impl Default for Buffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn buf(s: &str) -> Buffer {
        let mut b = Buffer::new();
        b.gap = GapBuffer::from_str(s);
        b
    }

    #[test]
    fn insert_char_at_cursor() {
        let mut b = buf("hello");
        b.move_cursor(5);
        b.insert_char('!');
        assert_eq!(b.text(), "hello!");
        assert_eq!(b.cursor, 6);
    }

    #[test]
    fn replace_mode() {
        let mut b = buf("hello");
        b.insert_mode = InsertMode::Replace;
        b.move_cursor(0);
        b.insert_char('H');
        assert_eq!(b.text(), "Hello");
    }

    #[test]
    fn backspace_and_delete() {
        let mut b = buf("hello");
        b.move_cursor(5);
        b.backspace();
        assert_eq!(b.text(), "hell");
        b.move_cursor(0);
        b.delete_forward();
        assert_eq!(b.text(), "ell");
    }

    #[test]
    fn selection_delete() {
        let mut b = buf("hello world");
        b.selection = Some(Selection::new(5, 11));
        b.delete_selection();
        assert_eq!(b.text(), "hello");
        assert_eq!(b.cursor, 5);
    }

    #[test]
    fn insert_pair_places_cursor_between() {
        let mut b = buf("");
        b.insert_pair('(', ')');
        assert_eq!(b.text(), "()");
        assert_eq!(b.cursor, 1);
        b.insert_pair('[', ']');
        assert_eq!(b.text(), "([])");
        assert_eq!(b.cursor, 2);
    }

    #[test]
    fn insert_pair_replace_mode() {
        let mut b = buf("ab");
        b.insert_mode = InsertMode::Replace;
        b.move_cursor(0);
        b.insert_pair('(', ')');
        assert_eq!(b.text(), "()b");
        assert_eq!(b.cursor, 1);
        b.undo();
        assert_eq!(b.text(), "ab");
        assert_eq!(b.cursor, 0);
        b.redo();
        assert_eq!(b.text(), "()b");
        assert_eq!(b.cursor, 1);
    }

    #[test]
    fn insert_pair_over_selection() {
        let mut b = buf("hello world");
        b.selection = Some(Selection::new(6, 11));
        b.move_cursor(11);
        b.insert_pair('(', ')');
        assert_eq!(b.text(), "hello (world)");
        assert_eq!(b.cursor, 12);
        b.undo();
        assert_eq!(b.text(), "hello world");
        assert_eq!(b.cursor, 11);
        b.redo();
        assert_eq!(b.text(), "hello (world)");
    }

    #[test]
    fn should_pair_quote_heuristic() {
        let mut b = buf("don't");
        b.move_cursor(3);
        assert!(!b.should_pair_quote());
        b.move_cursor(0);
        assert!(b.should_pair_quote());
        let mut b2 = buf("foo bar");
        b2.move_cursor(4);
        assert!(b2.should_pair_quote());
        let mut b3 = buf("(foo");
        b3.move_cursor(1);
        assert!(b3.should_pair_quote());
    }

    #[test]
    fn typing_replaces_selection() {
        let mut b = buf("hello world");
        b.selection = Some(Selection::new(6, 11));
        b.move_cursor(6);
        b.insert_str("there");
        assert_eq!(b.text(), "hello there");
    }

    #[test]
    fn word_navigation() {
        let mut b = buf("the quick brown fox");
        b.move_cursor(0);
        b.forward_word();
        assert_eq!(b.cursor, 4);
        b.forward_word();
        assert_eq!(b.cursor, 10);
        b.backward_word();
        assert_eq!(b.cursor, 4);
    }

    #[test]
    fn select_word_and_line() {
        let mut b = buf("one two\nthree");
        b.move_cursor(4);
        b.select_word();
        assert_eq!(b.text_range(4, 7), "two");
        b.select_line();
        assert_eq!(b.text_range(0, 7), "one two");
    }

    #[test]
    fn move_selected_text() {
        let mut b = buf("abc def ghi");
        b.selection = Some(Selection::new(4, 7));
        b.move_cursor(4);
        b.move_selection_word(1);
        assert_eq!(b.text(), "abc ghi def");
        b.move_selection_word(-1);
        assert_eq!(b.text(), "abc def ghi");
    }

    #[test]
    fn move_selected_line() {
        let mut b = buf("one\ntwo\nthree");
        b.selection = Some(Selection::new(4, 7));
        b.move_cursor(4);
        b.move_selection_line(1);
        assert_eq!(b.text(), "one\nthree\ntwo");
        b.move_selection_line(-1);
        assert_eq!(b.text(), "one\ntwo\nthree");
    }

    #[test]
    fn find_next_wraps() {
        let mut b = buf("foo bar foo");
        b.move_cursor(0);
        assert!(b.find_next("foo"));
        assert_eq!(b.cursor, 0);
        assert!(b.find_next("foo"));
        assert_eq!(b.cursor, 8);
        assert!(b.find_next("foo"));
        assert_eq!(b.cursor, 0);
    }

    #[test]
    fn find_next_overlapping() {
        let mut b = buf("aaa");
        b.move_cursor(0);
        assert!(b.find_next("aa"));
        assert_eq!(b.cursor, 0);
        assert!(b.find_next("aa"));
        assert_eq!(b.cursor, 1);
        assert!(b.find_next("aa"));
        assert_eq!(b.cursor, 0);
    }

    #[test]
    fn find_next_multibyte() {
        let mut b = buf("héllo wörld foo");
        b.move_cursor(0);
        assert!(b.find_next("wörld"));
        assert_eq!(b.cursor, 6);
        assert_eq!(b.find_match, Some((6, 11)));
    }

    #[test]
    fn goto_lines() {
        let mut b = buf("a\nb\nc");
        b.move_cursor(0);
        b.move_to_line(2);
        assert_eq!(b.cursor, 4);
        b.goto_bottom();
        assert_eq!(b.cursor, 5);
        b.goto_top();
        assert_eq!(b.cursor, 0);
    }

    #[test]
    fn undo_insert() {
        let mut b = buf("hello");
        b.move_cursor(5);
        b.insert_char('!');
        assert_eq!(b.text(), "hello!");
        b.undo();
        assert_eq!(b.text(), "hello");
        assert_eq!(b.cursor, 5);
        b.redo();
        assert_eq!(b.text(), "hello!");
        assert_eq!(b.cursor, 6);
    }

    #[test]
    fn undo_backspace() {
        let mut b = buf("hello");
        b.move_cursor(5);
        b.backspace();
        assert_eq!(b.text(), "hell");
        b.undo();
        assert_eq!(b.text(), "hello");
        assert_eq!(b.cursor, 5);
        b.redo();
        assert_eq!(b.text(), "hell");
    }

    #[test]
    fn undo_delete_forward() {
        let mut b = buf("hello");
        b.move_cursor(0);
        b.delete_forward();
        assert_eq!(b.text(), "ello");
        b.undo();
        assert_eq!(b.text(), "hello");
        b.redo();
        assert_eq!(b.text(), "ello");
    }

    #[test]
    fn undo_delete_selection() {
        let mut b = buf("hello world");
        b.selection = Some(Selection::new(5, 11));
        b.move_cursor(11);
        b.delete_selection();
        assert_eq!(b.text(), "hello");
        b.undo();
        assert_eq!(b.text(), "hello world");
        assert_eq!(b.cursor, 11);
    }

    #[test]
    fn undo_replace_mode() {
        let mut b = buf("hello");
        b.insert_mode = InsertMode::Replace;
        b.move_cursor(0);
        b.insert_char('H');
        assert_eq!(b.text(), "Hello");
        b.undo();
        assert_eq!(b.text(), "hello");
        b.redo();
        assert_eq!(b.text(), "Hello");
    }

    #[test]
    fn undo_move_text() {
        let mut b = buf("abc def ghi");
        b.selection = Some(Selection::new(4, 7));
        b.move_cursor(4);
        b.move_selection_word(1);
        assert_eq!(b.text(), "abc ghi def");
        b.undo();
        assert_eq!(b.text(), "abc def ghi");
        b.redo();
        assert_eq!(b.text(), "abc ghi def");
    }

    #[test]
    fn typing_coalesces_into_one_undo() {
        let mut b = buf("");
        b.insert_str("abc");
        assert_eq!(b.undo_stack.len(), 1);
        b.undo();
        assert_eq!(b.text(), "");
        b.redo();
        assert_eq!(b.text(), "abc");
    }

    #[test]
    fn backspace_coalesces_into_one_undo() {
        let mut b = buf("hello");
        b.move_cursor(5);
        b.backspace();
        b.backspace();
        assert_eq!(b.undo_stack.len(), 1);
        b.undo();
        assert_eq!(b.text(), "hello");
    }

    #[test]
    fn new_edit_clears_redo() {
        let mut b = buf("hello");
        b.move_cursor(5);
        b.insert_char('!');
        b.undo();
        assert_eq!(b.redo_stack.len(), 1);
        b.insert_char('?');
        assert_eq!(b.redo_stack.len(), 0);
        b.undo();
        assert_eq!(b.text(), "hello");
    }

    #[test]
    fn undo_restores_cursor() {
        let mut b = buf("hello world");
        b.move_cursor(5);
        b.insert_str(" there");
        assert_eq!(b.cursor, 11);
        b.undo();
        assert_eq!(b.cursor, 5);
        b.redo();
        assert_eq!(b.cursor, 11);
    }

    #[test]
    fn undo_blocked_on_read_only() {
        let mut b = Buffer::from_tutorial("hello");
        b.move_cursor(5);
        b.insert_char('!');
        assert_eq!(b.text(), "hello");
        b.undo();
        assert_eq!(b.text(), "hello");
    }

    #[test]
    fn undo_typing_over_selection_restores_selection() {
        let mut b = buf("hello world");
        b.selection = Some(Selection::new(6, 11));
        b.move_cursor(11);
        b.insert_str("there");
        assert_eq!(b.text(), "hello there");
        b.undo();
        assert_eq!(b.text(), "hello world");
        assert_eq!(b.cursor, 11);
        b.redo();
        assert_eq!(b.text(), "hello there");
    }

    #[test]
    fn undo_backspace_mid_line() {
        let mut b = buf("hello world");
        b.move_cursor(6);
        b.backspace();
        b.backspace();
        assert_eq!(b.text(), "hellworld");
        b.undo();
        assert_eq!(b.text(), "hello world");
        assert_eq!(b.cursor, 6);
    }

    #[test]
    fn undo_three_backspaces_coalesce() {
        let mut b = buf("hello world");
        b.move_cursor(6);
        b.backspace();
        b.backspace();
        b.backspace();
        assert_eq!(b.undo_stack.len(), 1);
        b.undo();
        assert_eq!(b.text(), "hello world");
        assert_eq!(b.cursor, 6);
    }

    #[test]
    fn delete_forward_coalesces() {
        let mut b = buf("hello world");
        b.move_cursor(0);
        b.delete_forward();
        b.delete_forward();
        assert_eq!(b.undo_stack.len(), 1);
        assert_eq!(b.text(), "llo world");
        b.undo();
        assert_eq!(b.text(), "hello world");
        assert_eq!(b.cursor, 0);
    }

    #[test]
    fn undo_move_selection_line() {
        let mut b = buf("one\ntwo\nthree");
        b.selection = Some(Selection::new(4, 7));
        b.move_cursor(7);
        b.move_selection_line(1);
        assert_eq!(b.text(), "one\nthree\ntwo");
        b.undo();
        assert_eq!(b.text(), "one\ntwo\nthree");
        assert_eq!(b.cursor, 7);
        b.redo();
        assert_eq!(b.text(), "one\nthree\ntwo");
    }

    #[test]
    fn dirty_flag_after_redo() {
        let mut b = buf("hello");
        b.move_cursor(5);
        b.insert_char('!');
        b.dirty = false;
        b.undo();
        assert!(!b.dirty);
        b.redo();
        assert!(b.dirty);
    }

    #[test]
    fn undo_replace_mode_over_selection() {
        let mut b = buf("hello world");
        b.insert_mode = InsertMode::Replace;
        b.selection = Some(Selection::new(6, 11));
        b.move_cursor(11);
        b.insert_char('X');
        assert_eq!(b.text(), "hello X");
        b.undo();
        assert_eq!(b.text(), "hello world");
    }

    #[test]
    fn undo_replace_mode_over_selection_with_trailing_text() {
        let mut b = buf("hello world foo");
        b.insert_mode = InsertMode::Replace;
        b.selection = Some(Selection::new(6, 11));
        b.move_cursor(11);
        b.insert_char('X');
        assert_eq!(b.text(), "hello Xfoo");
        b.undo();
        assert_eq!(b.text(), "hello world foo");
        b.redo();
        assert_eq!(b.text(), "hello Xfoo");
    }

    #[test]
    fn save_preserves_permissions_and_symlinks() {
        let dir = std::env::temp_dir().join(format!("txtwrk-save-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let real = dir.join("real.txt");
        fs::write(&real, "old").unwrap();
        fs::set_permissions(&real, fs::Permissions::from_mode(0o600)).unwrap();
        let link = dir.join("link.txt");
        std::os::unix::fs::symlink(&real, &link).unwrap();

        let mut b = Buffer::from_file(&link).unwrap();
        b.move_cursor(b.len());
        b.insert_str("new");
        b.save().unwrap();

        assert!(
            fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(fs::read_to_string(&real).unwrap(), "oldnew");
        let mode = fs::metadata(&real).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        let _ = fs::remove_dir_all(&dir);
    }
}
