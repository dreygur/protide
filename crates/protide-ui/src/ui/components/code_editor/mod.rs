//! Code editor component with syntax highlighting
//!
//! A reusable editor component inspired by Zed's architecture.
//! Supports both editable (body editor) and read-only (response viewer) modes.

mod buffer;
mod highlight;
mod selection;

pub use buffer::TextBuffer;
pub use highlight::{Highlighter, Language, PlainHighlighter};
pub use selection::Selection;

use std::collections::{HashMap, HashSet};

use gpui::{
    div, font, prelude::*, px, canvas, ClipboardItem, Context, FocusHandle,
    IntoElement, KeyDownEvent, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    ParentElement, Render, ScrollWheelEvent, StyledText, TextRun, Styled, Window, Bounds, Pixels,
};

use crate::theme;

/// Configuration for CodeEditor
#[derive(Clone)]
pub struct Config {
    pub read_only: bool,
    pub show_line_numbers: bool,
    pub font_size: f32,
    pub line_height: f32,
    pub gutter_width: f32,
    pub font_family: String,
    pub char_width_ratio: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            read_only: false,
            show_line_numbers: true,
            font_size: 13.0,
            line_height: 20.0,
            gutter_width: 55.0, // Includes fold marker space
            font_family: "JetBrains Mono".to_string(),
            char_width_ratio: 0.602, // More precise ratio for monospace fonts
        }
    }
}

/// Undo/redo state snapshot
#[derive(Clone)]
struct UndoState {
    content: String,
    selection: Selection,
}

/// Code editor with syntax highlighting and optional editing
pub struct CodeEditor {
    buffer: TextBuffer,
    selection: Selection,
    language: Language,
    highlighter: Box<dyn Highlighter>,
    scroll_offset: f32,
    focus_handle: FocusHandle,
    config: Config,
    bounds: Option<Bounds<Pixels>>,
    is_dragging: bool,
    // Folding state
    fold_ranges: HashMap<usize, usize>, // start_line -> end_line (inclusive)
    folded_lines: HashSet<usize>,       // Lines that are currently folded (hidden)
    // Undo/redo stacks
    undo_stack: Vec<UndoState>,
    redo_stack: Vec<UndoState>,
}

impl CodeEditor {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            buffer: TextBuffer::default(),
            selection: Selection::cursor(0),
            language: Language::Plain,
            highlighter: Box::new(PlainHighlighter),
            scroll_offset: 0.0,
            focus_handle: cx.focus_handle(),
            config: Config::default(),
            bounds: None,
            is_dragging: false,
            fold_ranges: HashMap::new(),
            folded_lines: HashSet::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    /// Set initial content
    pub fn with_content(mut self, text: &str) -> Self {
        self.buffer = TextBuffer::new(text.to_string());
        self.compute_fold_ranges();
        self
    }

    /// Set language for syntax highlighting
    pub fn with_language(mut self, lang: Language) -> Self {
        self.language = lang;
        self.highlighter = lang.highlighter();
        self.compute_fold_ranges();
        self
    }

    /// Set read-only mode
    pub fn with_read_only(mut self, read_only: bool) -> Self {
        self.config.read_only = read_only;
        self
    }

    /// Configure line numbers visibility
    pub fn with_line_numbers(mut self, show: bool) -> Self {
        self.config.show_line_numbers = show;
        self
    }

    /// Set font size
    #[allow(dead_code)]
    pub fn with_font_size(mut self, size: f32) -> Self {
        self.config.font_size = size;
        self.config.line_height = size * 1.5;
        self
    }

    /// Get current content
    pub fn content(&self) -> &str {
        self.buffer.content()
    }

    /// Set content programmatically
    pub fn set_content(&mut self, text: &str, cx: &mut Context<Self>) {
        self.buffer.set_content(text.to_string());
        self.selection = Selection::cursor(0);
        self.scroll_offset = 0.0;
        cx.notify();
    }

    /// Auto-detect language from content
    #[allow(dead_code)]
    pub fn detect_language(&mut self, cx: &mut Context<Self>) {
        let lang = Language::detect(self.buffer.content());
        if lang != self.language {
            self.language = lang;
            self.highlighter = lang.highlighter();
            cx.notify();
        }
    }

    /// Set language programmatically
    pub fn set_language(&mut self, lang: Language, cx: &mut Context<Self>) {
        if lang != self.language {
            self.language = lang;
            self.highlighter = lang.highlighter();
            self.compute_fold_ranges();
            cx.notify();
        }
    }

    /// Get focus handle
    #[allow(dead_code)]
    pub fn focus_handle(&self) -> &FocusHandle {
        &self.focus_handle
    }

    // === Folding ===

    /// Compute fold ranges from content (finds matching { } and [ ])
    fn compute_fold_ranges(&mut self) {
        self.fold_ranges.clear();
        let mut stack: Vec<(usize, char)> = Vec::new(); // (line_idx, bracket_char)

        for line_idx in 0..self.buffer.line_count() {
            let line = self.buffer.line(line_idx).unwrap_or("");
            for c in line.chars() {
                match c {
                    '{' | '[' => {
                        stack.push((line_idx, c));
                    }
                    '}' => {
                        if let Some((start_line, '{')) = stack.pop() {
                            if start_line < line_idx {
                                self.fold_ranges.insert(start_line, line_idx);
                            }
                        }
                    }
                    ']' => {
                        if let Some((start_line, '[')) = stack.pop() {
                            if start_line < line_idx {
                                self.fold_ranges.insert(start_line, line_idx);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    /// Toggle fold state for a line (if it's a fold start)
    fn toggle_fold(&mut self, line_idx: usize, cx: &mut Context<Self>) {
        if let Some(&end_line) = self.fold_ranges.get(&line_idx) {
            // Check if currently folded
            let is_folded = self.folded_lines.contains(&(line_idx + 1));

            if is_folded {
                // Unfold: remove all lines in range from folded set
                for l in (line_idx + 1)..=end_line {
                    self.folded_lines.remove(&l);
                }
            } else {
                // Fold: add all lines in range to folded set
                for l in (line_idx + 1)..=end_line {
                    self.folded_lines.insert(l);
                }
            }
            cx.notify();
        }
    }

    /// Check if a line is hidden due to folding
    fn is_line_visible(&self, line_idx: usize) -> bool {
        !self.folded_lines.contains(&line_idx)
    }

    /// Check if a line can be folded (is a fold start)
    fn is_foldable(&self, line_idx: usize) -> bool {
        self.fold_ranges.contains_key(&line_idx)
    }

    /// Check if a fold region starting at line_idx is currently folded
    fn is_folded(&self, line_idx: usize) -> bool {
        self.fold_ranges.get(&line_idx)
            .is_some_and(|_| self.folded_lines.contains(&(line_idx + 1)))
    }

    /// Fold all regions
    #[allow(dead_code)]
    pub fn fold_all(&mut self, cx: &mut Context<Self>) {
        for (&start, &end) in &self.fold_ranges.clone() {
            for l in (start + 1)..=end {
                self.folded_lines.insert(l);
            }
        }
        cx.notify();
    }

    /// Unfold all regions
    #[allow(dead_code)]
    pub fn unfold_all(&mut self, cx: &mut Context<Self>) {
        self.folded_lines.clear();
        cx.notify();
    }

    /// Beautify/format the content (currently supports JSON)
    pub fn beautify(&mut self, cx: &mut Context<Self>) {
        if self.config.read_only {
            return;
        }

        match self.language {
            Language::Json => {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(self.buffer.content()) {
                    if let Ok(formatted) = serde_json::to_string_pretty(&value) {
                        self.buffer = TextBuffer::new(formatted);
                        self.selection = Selection::cursor(0);
                        self.compute_fold_ranges();
                        self.folded_lines.clear();
                        cx.notify();
                    }
                }
            }
            _ => {} // No beautify for other languages yet
        }
    }

    /// Check if focused
    pub fn is_focused(&self, window: &Window) -> bool {
        self.focus_handle.is_focused(window)
    }

    // === Undo/Redo ===

    /// Save current state before making changes
    fn save_undo_state(&mut self) {
        self.undo_stack.push(UndoState {
            content: self.buffer.content().to_string(),
            selection: self.selection.clone(),
        });
        // Clear redo stack when making new changes
        self.redo_stack.clear();
        // Limit stack size to prevent memory issues
        if self.undo_stack.len() > 100 {
            self.undo_stack.remove(0);
        }
    }

    /// Undo last change
    fn undo(&mut self, cx: &mut Context<Self>) {
        if let Some(state) = self.undo_stack.pop() {
            // Save current state to redo stack
            self.redo_stack.push(UndoState {
                content: self.buffer.content().to_string(),
                selection: self.selection.clone(),
            });
            // Restore previous state
            self.buffer.set_content(state.content);
            self.selection = state.selection;
            self.compute_fold_ranges();
            cx.notify();
        }
    }

    /// Redo last undone change
    fn redo(&mut self, cx: &mut Context<Self>) {
        if let Some(state) = self.redo_stack.pop() {
            // Save current state to undo stack
            self.undo_stack.push(UndoState {
                content: self.buffer.content().to_string(),
                selection: self.selection.clone(),
            });
            // Restore redo state
            self.buffer.set_content(state.content);
            self.selection = state.selection;
            self.compute_fold_ranges();
            cx.notify();
        }
    }

    // === Text Operations ===

    fn insert_text(&mut self, text: &str, cx: &mut Context<Self>) {
        if self.config.read_only {
            return;
        }

        self.save_undo_state();

        let range = self.selection.range();
        if !range.is_empty() {
            self.buffer.delete(range.clone());
            self.selection = Selection::cursor(range.start);
        }

        let offset = self.selection.head;
        self.buffer.insert(offset, text);
        self.selection.adjust_for_insert(offset, text.len());
        self.selection.collapse_to_head();
        cx.notify();
    }

    fn delete_selection(&mut self, cx: &mut Context<Self>) {
        if self.config.read_only {
            return;
        }

        let range = self.selection.range();
        if range.is_empty() {
            return;
        }

        self.save_undo_state();
        self.buffer.delete(range.clone());
        self.selection = Selection::cursor(range.start);
        cx.notify();
    }

    fn delete_char(&mut self, forward: bool, cx: &mut Context<Self>) {
        if self.config.read_only {
            return;
        }

        if !self.selection.is_empty() {
            self.delete_selection(cx);
            return;
        }

        self.save_undo_state();

        let pos = self.selection.head;
        if forward {
            if pos < self.buffer.len() {
                self.buffer.delete(pos..pos + 1);
            }
        } else {
            if pos > 0 {
                self.buffer.delete(pos - 1..pos);
                self.selection = Selection::cursor(pos - 1);
            }
        }
        cx.notify();
    }

    // === Cursor Movement ===

    fn move_cursor(&mut self, new_pos: usize, extend: bool, cx: &mut Context<Self>) {
        self.selection.move_to(new_pos.min(self.buffer.len()), extend);
        cx.notify();
    }

    fn cursor_left(&mut self, extend: bool, cx: &mut Context<Self>) {
        let pos = if extend || self.selection.is_empty() {
            self.selection.head.saturating_sub(1)
        } else {
            self.selection.start()
        };
        self.move_cursor(pos, extend, cx);
    }

    fn cursor_right(&mut self, extend: bool, cx: &mut Context<Self>) {
        let pos = if extend || self.selection.is_empty() {
            (self.selection.head + 1).min(self.buffer.len())
        } else {
            self.selection.end()
        };
        self.move_cursor(pos, extend, cx);
    }

    fn cursor_up(&mut self, extend: bool, cx: &mut Context<Self>) {
        let (line, col) = self.buffer.offset_to_point(self.selection.head);
        if line > 0 {
            let new_pos = self.buffer.point_to_offset(line - 1, col);
            self.move_cursor(new_pos, extend, cx);
        }
    }

    fn cursor_down(&mut self, extend: bool, cx: &mut Context<Self>) {
        let (line, col) = self.buffer.offset_to_point(self.selection.head);
        if line < self.buffer.line_count().saturating_sub(1) {
            let new_pos = self.buffer.point_to_offset(line + 1, col);
            self.move_cursor(new_pos, extend, cx);
        }
    }

    fn cursor_home(&mut self, extend: bool, cx: &mut Context<Self>) {
        let (line, _) = self.buffer.offset_to_point(self.selection.head);
        let line_start = self.buffer.line_start(line);
        self.move_cursor(line_start, extend, cx);
    }

    fn cursor_end(&mut self, extend: bool, cx: &mut Context<Self>) {
        let (line, _) = self.buffer.offset_to_point(self.selection.head);
        let line_end = self.buffer.line_end(line);
        self.move_cursor(line_end, extend, cx);
    }

    // === Clipboard ===

    fn copy(&self, cx: &mut Context<Self>) {
        let range = self.selection.range();
        if range.is_empty() {
            return;
        }
        let text = &self.buffer.content()[range];
        cx.write_to_clipboard(ClipboardItem::new_string(text.to_string()));
    }

    fn cut(&mut self, cx: &mut Context<Self>) {
        if self.config.read_only {
            return;
        }
        self.copy(cx);
        self.delete_selection(cx);
    }

    fn paste(&mut self, cx: &mut Context<Self>) {
        if self.config.read_only {
            return;
        }
        if let Some(item) = cx.read_from_clipboard() {
            if let Some(text) = item.text() {
                self.insert_text(&text, cx);
            }
        }
    }

    fn select_all(&mut self, cx: &mut Context<Self>) {
        self.selection = Selection::new(0, self.buffer.len());
        cx.notify();
    }

    // === Position Calculation ===

    fn offset_at_position(&self, x: f32, y: f32) -> usize {
        let char_width = self.config.font_size * self.config.char_width_ratio;
        let line_height = self.config.line_height;
        let gutter = if self.config.show_line_numbers { self.config.gutter_width } else { 0.0 };
        let spacer = 8.0; // Left spacer width

        // Subtract gutter and spacer to get position relative to text area
        let adjusted_x = (x - gutter - spacer).max(0.0);
        let adjusted_y = y + self.scroll_offset;

        let line = (adjusted_y / line_height) as usize;
        let col = (adjusted_x / char_width) as usize;

        self.buffer.point_to_offset(line, col)
    }
}

impl Render for CodeEditor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let is_focused = self.is_focused(window);
        let entity = cx.entity().clone();
        let show_beautify = self.language == Language::Json && !self.config.read_only;

        div()
            .id("code-editor")
            .size_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .bg(theme.colors.bg_secondary)
            .border_1()
            .border_color(if is_focused { theme.colors.accent } else { gpui::transparent_white() })
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(Self::handle_key_down))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::handle_mouse_down))
            .on_mouse_move(cx.listener(Self::handle_mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::handle_mouse_up))
            .on_scroll_wheel(cx.listener(Self::handle_scroll))
            .child(
                canvas(
                    move |bounds, _window, cx| {
                        let _ = entity.update(cx, |this, _| {
                            this.bounds = Some(bounds);
                        });
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full()
            )
            // Toolbar with beautify button (top right)
            .when(show_beautify, |el| {
                el.child(
                    div()
                        .absolute()
                        .top(px(4.0))
                        .right(px(4.0))
                        .child(
                            div()
                                .id("btn-beautify")
                                .px(px(8.0))
                                .py(px(4.0))
                                .bg(theme.colors.bg_tertiary)
                                .text_size(px(11.0))
                                .text_color(theme.colors.text_muted)
                                .cursor_pointer()
                                .hover(|s| s.bg(theme.colors.accent).text_color(gpui::white()))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.beautify(cx);
                                }))
                                .child("{ } Beautify")
                        )
                )
            })
            .child(self.render_editor(window, cx))
    }
}

impl CodeEditor {
    fn render_editor(&self, _window: &Window, cx: &Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let line_height = self.config.line_height;
        let font_size = self.config.font_size;

        let visible_lines: Vec<usize> = (0..self.buffer.line_count())
            .filter(|&idx| self.is_line_visible(idx))
            .collect();
        let total_visible = visible_lines.len();

        let viewport_height = self.bounds.map(|b| f32::from(b.size.height)).unwrap_or(400.0);
        let first = (self.scroll_offset / line_height) as usize;
        let count = (viewport_height / line_height).ceil() as usize + 2;
        let last = (first + count).min(total_visible);

        // Fractional sub-line offset for smooth pixel scrolling.
        // mt(-frac) shifts the content up so line `first` appears at y = -frac,
        // which is correctly clipped by overflow_hidden on the parent.
        let frac = self.scroll_offset % line_height;

        div()
            .id("code-editor-content")
            .size_full()
            .overflow_hidden()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .min_w_full()
                    .mt(px(-frac))
                    .children(visible_lines[first..last].iter().map(|&line_idx| {
                        self.render_line(line_idx, &theme.colors, line_height, font_size, cx)
                    }))
            )
    }

    fn render_line(&self, line_idx: usize, theme: &theme::Colors, line_height: f32, font_size: f32, cx: &Context<Self>) -> impl IntoElement {
        let line_content = self.buffer.line(line_idx).unwrap_or("");
        let tokens = self.highlighter.tokenize_line(line_content);
        let line_start = self.buffer.line_start(line_idx);
        let line_end = self.buffer.line_end(line_idx);
        let sel_range = self.selection.range();

        // Folding state
        let is_foldable = self.is_foldable(line_idx);
        let is_folded = self.is_folded(line_idx);

        // Calculate selection within this line
        let sel_start_in_line = sel_range.start.saturating_sub(line_start);
        let sel_end_in_line = sel_range.end.min(line_end + 1).saturating_sub(line_start);
        let has_selection = sel_range.start <= line_end && sel_range.end > line_start;

        // Cursor position
        let cursor_pos = self.selection.head;
        let cursor_in_line = cursor_pos >= line_start && cursor_pos <= line_end;
        let cursor_col = if cursor_in_line { cursor_pos - line_start } else { 0 };

        div()
            .h(px(line_height))
            .min_w_full()
            .flex()
            .items_center()
            .flex_shrink_0()
            .when(cursor_in_line && !self.config.read_only, |el| {
                el.bg(theme.bg_elevated.opacity(0.5))
            })
            // Gutter with fold marker and line number
            .when(self.config.show_line_numbers, |el| {
                el.child(
                    div()
                        .w(px(self.config.gutter_width))
                        .h_full()
                        .bg(theme.bg_primary)
                        .flex()
                        .items_center()
                        .flex_shrink_0()
                        // Fold marker
                        .child(
                            div()
                                .id(("fold-marker", line_idx))
                                .w(px(16.0))
                                .h_full()
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_size(px(10.0))
                                .text_color(theme.text_secondary)
                                .when(is_foldable, |el| {
                                    el.cursor_pointer()
                                        .hover(|s| s.text_color(theme.accent))
                                        .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                                            this.toggle_fold(line_idx, cx);
                                        }))
                                        .child(if is_folded { "▶" } else { "▼" })
                                })
                        )
                        // Line number
                        .child(
                            div()
                                .flex_1()
                                .h_full()
                                .flex()
                                .items_center()
                                .justify_end()
                                .pr(px(8.0))
                                .text_size(px(font_size - 1.0))
                                .text_color(theme.text_secondary)
                                .child(format!("{}", line_idx + 1))
                        )
                )
            })
            // Line content - use explicit spacer instead of padding for precise positioning
            .child(
                div()
                    .h_full()
                    .flex()
                    .items_center()
                    .child(
                        // Left spacer (8px gap)
                        div().w(px(8.0)).h_full().flex_shrink_0()
                    )
                    .child(
                        // Text content area with cursor/selection
                        div()
                            .h_full()
                            .flex()
                            .items_center()
                            .relative()
                            // Selection highlight
                            .when(has_selection && !self.selection.is_empty(), |el| {
                                let char_width = font_size * self.config.char_width_ratio;
                                let sel_x = sel_start_in_line as f32 * char_width;
                                let sel_width = (sel_end_in_line - sel_start_in_line) as f32 * char_width;
                                el.child(
                                    div()
                                        .absolute()
                                        .top_0()
                                        .left(px(sel_x))
                                        .h_full()
                                        .w(px(sel_width.max(2.0)))
                                        .bg(gpui::rgba(0x3366ff40))
                                )
                            })
                            // Cursor
                            .when(cursor_in_line && !self.config.read_only, |el| {
                                let char_width = font_size * self.config.char_width_ratio;
                                let cursor_x = cursor_col as f32 * char_width;
                                el.child(
                                    div()
                                        .absolute()
                                        .top(px(2.0))
                                        .left(px(cursor_x))
                                        .h(px(line_height - 4.0))
                                        .w(px(2.0))
                                        .bg(theme.accent)
                                )
                            })
                            // StyledText — single GPU text paint per line, one TextRun per token
                            .child({
                                let mono_font = font(self.config.font_family.as_str());
                                let runs: Vec<TextRun> = tokens.into_iter().map(|token| {
                                    TextRun {
                                        len: token.text.len(),
                                        font: mono_font.clone(),
                                        color: token.kind.color(theme),
                                        background_color: None,
                                        underline: None,
                                        strikethrough: None,
                                    }
                                }).collect();
                                div()
                                    .text_size(px(font_size))
                                    .child(StyledText::new(line_content.to_string()).with_runs(runs))
                            })
                    )
                    // Folded indicator
                    .when(is_folded, |el| {
                        el.child(
                            div()
                                .ml(px(4.0))
                                .px(px(4.0))
                                .bg(theme.bg_tertiary)
                                .text_size(px(10.0))
                                .text_color(theme.text_muted)
                                .child("...")
                        )
                    })
            )
    }

    // === Event Handlers ===

    fn handle_key_down(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let shift = event.keystroke.modifiers.shift;
        let ctrl = event.keystroke.modifiers.control;

        match event.keystroke.key.as_str() {
            "left" => self.cursor_left(shift, cx),
            "right" => self.cursor_right(shift, cx),
            "up" => self.cursor_up(shift, cx),
            "down" => self.cursor_down(shift, cx),
            "home" => self.cursor_home(shift, cx),
            "end" => self.cursor_end(shift, cx),
            "backspace" => self.delete_char(false, cx),
            "delete" => self.delete_char(true, cx),
            "enter" => self.insert_text("\n", cx),
            "tab" => self.insert_text("  ", cx),
            "space" => self.insert_text(" ", cx),
            "a" if ctrl => self.select_all(cx),
            "c" if ctrl => self.copy(cx),
            "x" if ctrl => self.cut(cx),
            "v" if ctrl => self.paste(cx),
            "z" if ctrl && shift => self.redo(cx),     // Ctrl+Shift+Z = redo
            "z" if ctrl => self.undo(cx),              // Ctrl+Z = undo
            "y" if ctrl => self.redo(cx),              // Ctrl+Y = redo
            "b" if ctrl && shift => self.beautify(cx), // Ctrl+Shift+B = beautify
            key if key.len() == 1 && !ctrl => {
                self.insert_text(key, cx);
            }
            _ => {}
        }
    }

    fn handle_mouse_down(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.focus_handle.focus(window, cx);

        if let Some(bounds) = &self.bounds {
            let local_x = f32::from(event.position.x) - f32::from(bounds.origin.x);
            let local_y = f32::from(event.position.y) - f32::from(bounds.origin.y);
            let offset = self.offset_at_position(local_x, local_y);

            match event.click_count {
                1 => {
                    self.selection = Selection::cursor(offset);
                    self.is_dragging = true;
                }
                2 => {
                    // Word selection
                    let content = self.buffer.content();
                    let start = find_word_start(content, offset);
                    let end = find_word_end(content, offset);
                    self.selection = Selection::new(start, end);
                }
                3 => {
                    // Line selection
                    let (line, _) = self.buffer.offset_to_point(offset);
                    let start = self.buffer.line_start(line);
                    let end = if line < self.buffer.line_count() - 1 {
                        self.buffer.line_start(line + 1)
                    } else {
                        self.buffer.len()
                    };
                    self.selection = Selection::new(start, end);
                }
                _ => {}
            }
            cx.notify();
        }
    }

    fn handle_mouse_move(&mut self, event: &MouseMoveEvent, _window: &mut Window, cx: &mut Context<Self>) {
        if !self.is_dragging {
            return;
        }

        if let Some(bounds) = &self.bounds {
            let local_x = f32::from(event.position.x) - f32::from(bounds.origin.x);
            let local_y = f32::from(event.position.y) - f32::from(bounds.origin.y);
            let offset = self.offset_at_position(local_x, local_y);
            if self.selection.head != offset {
                self.selection.head = offset;
                cx.notify();
            }
        }
    }

    fn handle_scroll(&mut self, event: &ScrollWheelEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let line_height = self.config.line_height;
        let delta = event.delta.pixel_delta(px(line_height)).y;
        let visible_line_count = (0..self.buffer.line_count())
            .filter(|&i| self.is_line_visible(i))
            .count();
        let total_height = visible_line_count as f32 * line_height;
        let viewport_height = self.bounds.map(|b| f32::from(b.size.height)).unwrap_or(400.0);
        let max_scroll = (total_height - viewport_height).max(0.0);
        let new_offset = (self.scroll_offset - f32::from(delta)).clamp(0.0, max_scroll);
        if new_offset != self.scroll_offset {
            self.scroll_offset = new_offset;
            cx.notify();
        }
    }

    fn handle_mouse_up(&mut self, _event: &MouseUpEvent, _window: &mut Window, _cx: &mut Context<Self>) {
        self.is_dragging = false;
    }
}

// Helper functions for word navigation
fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn find_word_start(text: &str, pos: usize) -> usize {
    if text.is_empty() || pos == 0 {
        return 0;
    }
    let chars: Vec<char> = text.chars().collect();
    let mut start = pos.min(chars.len().saturating_sub(1));

    while start > 0 && !is_word_char(chars[start]) {
        start -= 1;
    }
    while start > 0 && is_word_char(chars[start - 1]) {
        start -= 1;
    }
    start
}

fn find_word_end(text: &str, pos: usize) -> usize {
    if text.is_empty() {
        return 0;
    }
    let chars: Vec<char> = text.chars().collect();
    let mut end = pos.min(chars.len().saturating_sub(1));

    while end < chars.len() && !is_word_char(chars[end]) {
        end += 1;
    }
    while end < chars.len() && is_word_char(chars[end]) {
        end += 1;
    }
    end
}
