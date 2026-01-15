//! Cursor and selection handling for code editor

use std::ops::Range;

/// Selection with anchor (start) and head (cursor position)
#[derive(Clone, Copy, Debug, Default)]
pub struct Selection {
    pub anchor: usize,
    pub head: usize,
}

impl Selection {
    /// Create a selection at a single cursor position
    pub fn cursor(offset: usize) -> Self {
        Self { anchor: offset, head: offset }
    }

    /// Create a selection spanning a range
    pub fn new(anchor: usize, head: usize) -> Self {
        Self { anchor, head }
    }

    /// Get the selected range (normalized: start <= end)
    pub fn range(&self) -> Range<usize> {
        let start = self.anchor.min(self.head);
        let end = self.anchor.max(self.head);
        start..end
    }

    /// Check if selection is empty (just a cursor)
    pub fn is_empty(&self) -> bool {
        self.anchor == self.head
    }

    /// Get start of selection
    pub fn start(&self) -> usize {
        self.anchor.min(self.head)
    }

    /// Get end of selection
    pub fn end(&self) -> usize {
        self.anchor.max(self.head)
    }

    /// Move cursor, optionally extending selection
    pub fn move_to(&mut self, offset: usize, extend: bool) {
        if extend {
            self.head = offset;
        } else {
            self.anchor = offset;
            self.head = offset;
        }
    }

    /// Collapse selection to cursor at head position
    pub fn collapse_to_head(&mut self) {
        self.anchor = self.head;
    }

    /// Collapse selection to cursor at start
    #[allow(dead_code)]
    pub fn collapse_to_start(&mut self) {
        let start = self.start();
        self.anchor = start;
        self.head = start;
    }

    /// Collapse selection to cursor at end
    #[allow(dead_code)]
    pub fn collapse_to_end(&mut self) {
        let end = self.end();
        self.anchor = end;
        self.head = end;
    }

    /// Check if offset is within selection
    #[allow(dead_code)]
    pub fn contains(&self, offset: usize) -> bool {
        let range = self.range();
        offset >= range.start && offset < range.end
    }

    /// Adjust selection after text insertion
    pub fn adjust_for_insert(&mut self, at: usize, len: usize) {
        if self.anchor >= at {
            self.anchor += len;
        }
        if self.head >= at {
            self.head += len;
        }
    }

    /// Adjust selection after text deletion
    #[allow(dead_code)]
    pub fn adjust_for_delete(&mut self, range: Range<usize>) {
        let len = range.end - range.start;

        // Adjust anchor
        if self.anchor > range.end {
            self.anchor -= len;
        } else if self.anchor > range.start {
            self.anchor = range.start;
        }

        // Adjust head
        if self.head > range.end {
            self.head -= len;
        } else if self.head > range.start {
            self.head = range.start;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor() {
        let sel = Selection::cursor(5);
        assert_eq!(sel.anchor, 5);
        assert_eq!(sel.head, 5);
        assert!(sel.is_empty());
    }

    #[test]
    fn test_range() {
        let sel = Selection::new(10, 5);
        assert_eq!(sel.range(), 5..10);
        assert_eq!(sel.start(), 5);
        assert_eq!(sel.end(), 10);
    }

    #[test]
    fn test_move_to() {
        let mut sel = Selection::cursor(5);
        sel.move_to(10, false);
        assert_eq!(sel.anchor, 10);
        assert_eq!(sel.head, 10);

        sel.move_to(15, true);
        assert_eq!(sel.anchor, 10);
        assert_eq!(sel.head, 15);
    }

    #[test]
    fn test_adjust_for_insert() {
        let mut sel = Selection::new(5, 10);
        sel.adjust_for_insert(7, 3);
        assert_eq!(sel.anchor, 5);
        assert_eq!(sel.head, 13);
    }

    #[test]
    fn test_adjust_for_delete() {
        let mut sel = Selection::new(5, 15);
        sel.adjust_for_delete(8..12);
        assert_eq!(sel.anchor, 5);
        assert_eq!(sel.head, 11);
    }
}
