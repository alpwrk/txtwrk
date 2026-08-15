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

    pub fn selection_range(&self) -> Option<(usize, usize)> {
        self.selection.map(|s| (s.start(), s.end()))
    }

    pub fn insert_char(&mut self, c: char) {
        if self.read_only {
            return;
        }
        if let Some(sel) = self.selection {
            self.gap.delete_range(sel.range());
            self.cursor = sel.start();
            self.selection = None;
        }
        match self.insert_mode {
            InsertMode::Insert => {
                self.gap.insert(self.cursor, c);
                self.cursor += 1;
            }
            InsertMode::Replace => {
                if self.cursor < self.len() {
                    self.gap
                        .replace_range(self.cursor..self.cursor + 1, &c.to_string());
                    self.cursor += 1;
                } else {
                    self.gap.insert(self.cursor, c);
                    self.cursor += 1;
                }
            }
        }
        self.dirty = true;
    }

    pub fn insert_str(&mut self, s: &str) {
        if self.read_only {
            return;
        }
        if let Some(sel) = self.selection {
            self.gap.delete_range(sel.range());
            self.cursor = sel.start();
            self.selection = None;
        }
        self.gap.insert_str(self.cursor, s);
        self.cursor += s.chars().count();
        self.dirty = true;
    }

    pub fn delete_selection(&mut self) {
        if self.read_only {
            return;
        }
        if let Some(sel) = self.selection {
            self.gap.delete_range(sel.range());
            self.cursor = sel.start();
            self.selection = None;
            self.dirty = true;
        }
    }

    pub fn backspace(&mut self) {
        if self.read_only {
            return;
        }
        if let Some(_sel) = self.selection {
            self.delete_selection();
            return;
        }
        if self.gap.delete_before(self.cursor).is_some() {
            self.cursor -= 1;
            self.dirty = true;
        }
    }

    pub fn delete_forward(&mut self) {
        if self.read_only {
            return;
        }
        if let Some(_sel) = self.selection {
            self.delete_selection();
            return;
        }
        if self.gap.delete_after(self.cursor).is_some() {
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

    pub fn move_selection_word(&mut self, dir: isize) {
        if self.read_only {
            return;
        }
        let Some(sel) = self.selection else {
            return;
        };
        let start = sel.start();
        let end = sel.end();
        let len = self.len();
        let is_ws = |c: char| c.is_whitespace();

        if dir > 0 {
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
            let sel_text = self.text_range(start, end);
            let following = self.text_range(word_start, word_end);
            let ws = self.text_range(end, word_start);
            let prefix = self.text_range(0, start);
            let suffix = self.text_range(word_end, len);
            let new = format!("{}{}{}{}", prefix, following, ws, sel_text);
            self.gap = GapBuffer::from_str(&format!("{}{}", new, suffix));
            let sel_start = start + following.chars().count() + ws.chars().count();
            let sel_end = sel_start + sel_text.chars().count();
            self.cursor = sel_end;
            self.selection = Some(Selection::new(sel_start, sel_end));
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
            let sel_text = self.text_range(start, end);
            let prev = self.text_range(word_start, word_end);
            let ws = self.text_range(word_end, start);
            let prefix = self.text_range(0, word_start);
            let suffix = self.text_range(end, len);
            let new = format!("{}{}{}{}", prefix, sel_text, ws, prev);
            self.gap = GapBuffer::from_str(&format!("{}{}", new, suffix));
            let sel_start = word_start;
            let sel_end = sel_start + sel_text.chars().count();
            self.cursor = sel_end;
            self.selection = Some(Selection::new(sel_start, sel_end));
        }
        self.dirty = true;
    }

    pub fn move_selection_line(&mut self, dir: isize) {
        if self.read_only {
            return;
        }
        let Some(sel) = self.selection else {
            return;
        };
        let start = sel.start();
        let end = sel.end();
        let len = self.len();

        if dir > 0 {
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
            let sel_text = self.text_range(start, end);
            let following = self.text_range(next_start, next_end);
            let ws = self.text_range(end, next_start);
            let prefix = self.text_range(0, start);
            let suffix = self.text_range(next_end_incl, len);
            let new = format!("{}{}{}{}", prefix, following, ws, sel_text);
            self.gap = GapBuffer::from_str(&format!("{}{}", new, suffix));
            let sel_start = start + following.chars().count() + ws.chars().count();
            let sel_end = sel_start + sel_text.chars().count();
            self.cursor = sel_end;
            self.selection = Some(Selection::new(sel_start, sel_end));
        } else {
            let line = self.gap.line_of(start);
            if line == 0 {
                return;
            }
            let prev_start = self.gap.line_start(line - 1);
            let prev_end = self.gap.line_end(line - 1);
            let sel_text = self.text_range(start, end);
            let prev = self.text_range(prev_start, prev_end);
            let ws = self.text_range(prev_end, start);
            let prefix = self.text_range(0, prev_start);
            let suffix = self.text_range(end, len);
            let new = format!("{}{}{}{}", prefix, sel_text, ws, prev);
            self.gap = GapBuffer::from_str(&format!("{}{}", new, suffix));
            let sel_start = prev_start;
            let sel_end = sel_start + sel_text.chars().count();
            self.cursor = sel_end;
            self.selection = Some(Selection::new(sel_start, sel_end));
        }
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
        let mut search_from = self.cursor;
        if let Some((ms, me)) = self.find_match
            && ms == self.cursor
        {
            search_from = me;
        }
        let mut wrapped = false;
        loop {
            if let Some(rel) = text[search_from..].find(query) {
                let abs = search_from + rel;
                if wrapped && abs >= self.cursor {
                    break;
                }
                self.cursor = abs;
                self.find_match = Some((abs, abs + query.len()));
                return true;
            }
            if wrapped {
                break;
            }
            search_from = 0;
            wrapped = true;
        }
        self.find_match = None;
        false
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
        fs::write(&path, self.text())?;
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
        fs::write(path, self.text())?;
        self.path = Some(path.to_path_buf());
        self.dirty = false;
        Ok(())
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
}
