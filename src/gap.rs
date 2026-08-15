use std::ops::Range;

const INITIAL_CAPACITY: usize = 64;

#[derive(Debug, Clone)]
pub struct GapBuffer {
    buf: Vec<char>,
    gap_start: usize,
    gap_end: usize,
}

impl GapBuffer {
    pub fn new() -> Self {
        Self {
            buf: Vec::with_capacity(INITIAL_CAPACITY),
            gap_start: 0,
            gap_end: 0,
        }
    }

    pub fn from_str(s: &str) -> Self {
        let mut gb = Self::new();
        gb.insert_str(0, s);
        gb
    }

    pub fn len(&self) -> usize {
        self.buf.len() - (self.gap_end - self.gap_start)
    }

    fn ensure_capacity(&mut self, extra: usize) {
        let gap_size = self.gap_end - self.gap_start;
        if gap_size >= extra {
            return;
        }
        let content_len = self.len();
        let new_gap = (gap_size * 2).max(extra).max(INITIAL_CAPACITY);
        let new_cap = content_len + new_gap;
        let mut new_buf = Vec::with_capacity(new_cap);
        new_buf.extend_from_slice(&self.buf[..self.gap_start]);
        new_buf.extend_from_slice(&self.buf[self.gap_end..]);
        new_buf.resize(new_cap, '\0');
        self.buf = new_buf;
        self.gap_start = content_len;
        self.gap_end = content_len + new_gap;
    }

    fn move_gap_to(&mut self, pos: usize) {
        let pos = pos.min(self.len());
        if pos == self.gap_start {
            return;
        }
        if pos < self.gap_start {
            let shift = self.gap_start - pos;
            let src = pos;
            let dst = self.gap_end - shift;
            self.buf.copy_within(src..src + shift, dst);
            self.gap_start = pos;
            self.gap_end = dst;
        } else {
            let shift = pos - self.gap_start;
            let src = self.gap_end;
            let dst = self.gap_start;
            self.buf.copy_within(src..src + shift, dst);
            self.gap_start = pos;
            self.gap_end = src + shift;
        }
    }

    pub fn insert(&mut self, pos: usize, c: char) {
        self.ensure_capacity(1);
        self.move_gap_to(pos);
        self.buf[self.gap_start] = c;
        self.gap_start += 1;
    }

    pub fn insert_str(&mut self, pos: usize, s: &str) {
        let chars: Vec<char> = s.chars().collect();
        if chars.is_empty() {
            return;
        }
        self.ensure_capacity(chars.len());
        self.move_gap_to(pos);
        for (i, c) in chars.iter().enumerate() {
            self.buf[self.gap_start + i] = *c;
        }
        self.gap_start += chars.len();
    }

    pub fn delete_before(&mut self, pos: usize) -> Option<char> {
        if pos == 0 {
            return None;
        }
        self.move_gap_to(pos);
        self.gap_start -= 1;
        Some(self.buf[self.gap_start])
    }

    pub fn delete_after(&mut self, pos: usize) -> Option<char> {
        if pos >= self.len() {
            return None;
        }
        self.move_gap_to(pos);
        self.gap_end += 1;
        Some(self.buf[self.gap_end - 1])
    }

    pub fn delete_range(&mut self, range: Range<usize>) {
        if range.start >= range.end {
            return;
        }
        self.move_gap_to(range.start);
        let mut remaining = range.end - range.start;
        while remaining > 0 {
            self.gap_end += 1;
            remaining -= 1;
        }
    }

    pub fn replace_range(&mut self, range: Range<usize>, s: &str) {
        let start = range.start;
        self.delete_range(range);
        self.insert_str(start, s);
    }

    pub fn char_at(&self, pos: usize) -> Option<char> {
        if pos >= self.len() {
            return None;
        }
        if pos < self.gap_start {
            Some(self.buf[pos])
        } else {
            Some(self.buf[pos + (self.gap_end - self.gap_start)])
        }
    }

    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        let mut s = String::with_capacity(self.len());
        s.extend(self.buf[..self.gap_start].iter());
        s.extend(self.buf[self.gap_end..].iter());
        s
    }

    pub fn iter(&self) -> impl Iterator<Item = char> + '_ {
        self.buf[..self.gap_start]
            .iter()
            .chain(self.buf[self.gap_end..].iter())
            .copied()
    }

    pub fn line_starts(&self) -> Vec<usize> {
        let mut starts = vec![0usize];
        for (i, c) in self.iter().enumerate() {
            if c == '\n' {
                starts.push(i + 1);
            }
        }
        starts
    }

    pub fn line_of(&self, pos: usize) -> usize {
        let starts = self.line_starts();
        match starts.binary_search(&pos) {
            Ok(i) => i,
            Err(i) => i - 1,
        }
    }

    pub fn line_start(&self, line: usize) -> usize {
        let starts = self.line_starts();
        starts.get(line).copied().unwrap_or(self.len())
    }

    pub fn line_end(&self, line: usize) -> usize {
        let starts = self.line_starts();
        let start = starts.get(line).copied().unwrap_or(self.len());
        let mut end = start;
        while let Some(c) = self.char_at(end) {
            if c == '\n' {
                break;
            }
            end += 1;
        }
        end
    }

    pub fn line_count(&self) -> usize {
        self.line_starts().len()
    }

    pub fn line_text(&self, line: usize) -> String {
        let start = self.line_start(line);
        let end = self.line_end(line);
        let mut s = String::new();
        for i in start..end {
            if let Some(c) = self.char_at(i) {
                s.push(c);
            }
        }
        s
    }
}

impl Default for GapBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_read() {
        let mut gb = GapBuffer::new();
        gb.insert_str(0, "hello");
        assert_eq!(gb.to_string(), "hello");
        gb.insert(5, '!');
        assert_eq!(gb.to_string(), "hello!");
    }

    #[test]
    fn insert_in_middle() {
        let mut gb = GapBuffer::from_str("hello world");
        gb.insert_str(5, ",");
        assert_eq!(gb.to_string(), "hello, world");
    }

    #[test]
    fn delete_before_and_after() {
        let mut gb = GapBuffer::from_str("abc");
        assert_eq!(gb.delete_before(1), Some('a'));
        assert_eq!(gb.to_string(), "bc");
        assert_eq!(gb.delete_after(0), Some('b'));
        assert_eq!(gb.to_string(), "c");
    }

    #[test]
    fn delete_range() {
        let mut gb = GapBuffer::from_str("hello world");
        gb.delete_range(5..11);
        assert_eq!(gb.to_string(), "hello");
    }

    #[test]
    fn replace_range() {
        let mut gb = GapBuffer::from_str("hello world");
        gb.replace_range(6..11, "there");
        assert_eq!(gb.to_string(), "hello there");
    }

    #[test]
    fn line_starts_and_text() {
        let mut gb = GapBuffer::from_str("one\ntwo\nthree");
        assert_eq!(gb.line_starts(), vec![0, 4, 8]);
        assert_eq!(gb.line_count(), 3);
        assert_eq!(gb.line_text(0), "one");
        assert_eq!(gb.line_text(1), "two");
        assert_eq!(gb.line_text(2), "three");
        assert_eq!(gb.line_of(5), 1);
        assert_eq!(gb.line_start(1), 4);
        assert_eq!(gb.line_end(1), 7);
    }

    #[test]
    fn edits_after_moves() {
        let mut gb = GapBuffer::from_str("abcdef");
        gb.move_gap_to(3);
        gb.insert(3, 'X');
        assert_eq!(gb.to_string(), "abcXdef");
        gb.move_gap_to(0);
        gb.insert(0, 'Z');
        assert_eq!(gb.to_string(), "ZabcXdef");
    }
}
