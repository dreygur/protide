//! Text buffer with line operations for code editor

use std::ops::Range;

/// Text buffer storing content with cached line start positions
pub struct TextBuffer {
    content: String,
    line_starts: Vec<usize>,
}

impl TextBuffer {
    pub fn new(content: String) -> Self {
        let mut buf = Self {
            content,
            line_starts: Vec::new(),
        };
        buf.recompute_line_starts();
        buf
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn len(&self) -> usize {
        self.content.len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    pub fn line(&self, idx: usize) -> Option<&str> {
        if idx >= self.line_starts.len() {
            return None;
        }
        let start = self.line_starts[idx];
        let end = if idx + 1 < self.line_starts.len() {
            self.line_starts[idx + 1].saturating_sub(1)
        } else {
            self.content.len()
        };
        Some(&self.content[start..end])
    }

    pub fn insert(&mut self, offset: usize, text: &str) {
        let offset = offset.min(self.content.len());
        self.content.insert_str(offset, text);
        self.recompute_line_starts();
    }

    pub fn delete(&mut self, range: Range<usize>) {
        let start = range.start.min(self.content.len());
        let end = range.end.min(self.content.len());
        if start < end {
            self.content.drain(start..end);
            self.recompute_line_starts();
        }
    }

    #[allow(dead_code)]
    pub fn replace(&mut self, range: Range<usize>, text: &str) {
        let start = range.start.min(self.content.len());
        let end = range.end.min(self.content.len());
        if start <= end {
            self.content.replace_range(start..end, text);
            self.recompute_line_starts();
        }
    }

    pub fn set_content(&mut self, content: String) {
        self.content = content;
        self.recompute_line_starts();
    }

    /// Convert byte offset to (line, column)
    pub fn offset_to_point(&self, offset: usize) -> (usize, usize) {
        let offset = offset.min(self.content.len());
        let line = self.line_starts
            .iter()
            .rposition(|&start| start <= offset)
            .unwrap_or(0);
        let col = offset - self.line_starts.get(line).copied().unwrap_or(0);
        (line, col)
    }

    /// Convert (line, column) to byte offset
    pub fn point_to_offset(&self, line: usize, col: usize) -> usize {
        if line >= self.line_starts.len() {
            return self.content.len();
        }
        let line_start = self.line_starts[line];
        let line_end = if line + 1 < self.line_starts.len() {
            self.line_starts[line + 1].saturating_sub(1)
        } else {
            self.content.len()
        };
        let max_col = line_end - line_start;
        line_start + col.min(max_col)
    }

    /// Get byte offset at start of line
    pub fn line_start(&self, line: usize) -> usize {
        self.line_starts.get(line).copied().unwrap_or(self.content.len())
    }

    /// Get byte offset at end of line (before newline)
    pub fn line_end(&self, line: usize) -> usize {
        if line >= self.line_starts.len() {
            return self.content.len();
        }
        if line + 1 < self.line_starts.len() {
            self.line_starts[line + 1].saturating_sub(1)
        } else {
            self.content.len()
        }
    }

    /// Previous valid char boundary before `pos` (steps back one char)
    pub fn prev_char_boundary(&self, pos: usize) -> usize {
        if pos == 0 { return 0; }
        let mut p = pos - 1;
        while p > 0 && !self.content.is_char_boundary(p) {
            p -= 1;
        }
        p
    }

    /// Next valid char boundary after `pos` (steps forward one char)
    pub fn next_char_boundary(&self, pos: usize) -> usize {
        let mut p = pos + 1;
        while p < self.content.len() && !self.content.is_char_boundary(p) {
            p += 1;
        }
        p.min(self.content.len())
    }

    fn recompute_line_starts(&mut self) {
        self.line_starts.clear();
        self.line_starts.push(0);
        for (i, c) in self.content.char_indices() {
            if c == '\n' {
                self.line_starts.push(i + 1);
            }
        }
    }
}

impl Default for TextBuffer {
    fn default() -> Self {
        Self::new(String::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_buffer() {
        let buf = TextBuffer::new(String::new());
        assert_eq!(buf.line_count(), 1);
        assert_eq!(buf.line(0), Some(""));
    }

    #[test]
    fn test_single_line() {
        let buf = TextBuffer::new("hello".into());
        assert_eq!(buf.line_count(), 1);
        assert_eq!(buf.line(0), Some("hello"));
    }

    #[test]
    fn test_multiline() {
        let buf = TextBuffer::new("line1\nline2\nline3".into());
        assert_eq!(buf.line_count(), 3);
        assert_eq!(buf.line(0), Some("line1"));
        assert_eq!(buf.line(1), Some("line2"));
        assert_eq!(buf.line(2), Some("line3"));
    }

    #[test]
    fn test_offset_to_point() {
        let buf = TextBuffer::new("ab\ncd\nef".into());
        assert_eq!(buf.offset_to_point(0), (0, 0));
        assert_eq!(buf.offset_to_point(1), (0, 1));
        assert_eq!(buf.offset_to_point(3), (1, 0));
        assert_eq!(buf.offset_to_point(6), (2, 0));
    }

    #[test]
    fn test_point_to_offset() {
        let buf = TextBuffer::new("ab\ncd\nef".into());
        assert_eq!(buf.point_to_offset(0, 0), 0);
        assert_eq!(buf.point_to_offset(1, 0), 3);
        assert_eq!(buf.point_to_offset(2, 1), 7);
    }

    #[test]
    fn test_insert() {
        let mut buf = TextBuffer::new("hello".into());
        buf.insert(5, " world");
        assert_eq!(buf.content(), "hello world");
    }

    #[test]
    fn test_delete() {
        let mut buf = TextBuffer::new("hello world".into());
        buf.delete(5..11);
        assert_eq!(buf.content(), "hello");
    }
}
