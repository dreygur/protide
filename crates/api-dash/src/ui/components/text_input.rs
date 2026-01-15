//! Reusable text input component with full editing support
//!
//! Features:
//! - Single/double/triple/quad click selection
//! - Mouse drag selection
//! - Keyboard navigation (arrows, home, end)
//! - Selection extension (Shift+Arrow)
//! - Copy/Cut/Paste (Ctrl+C/X/V)
//! - Undo/Redo (Ctrl+Z / Ctrl+Shift+Z)
//! - Select All (Ctrl+A)
//! - Customizable styling

#![allow(dead_code)]

use std::ops::Range;

/// Maximum number of undo states to keep
const MAX_UNDO_HISTORY: usize = 100;

use gpui::{
    div, prelude::*, px, ClipboardItem, Context, FocusHandle, Hsla, IntoElement,
    KeyDownEvent, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement,
    Render, SharedString, Styled, Window,
};

use crate::theme;

/// Check if character is a word character (alphanumeric or underscore)
pub fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Find the start of a word at the given position
pub fn find_word_start(text: &str, pos: usize) -> usize {
    if text.is_empty() || pos == 0 {
        return 0;
    }
    let chars: Vec<char> = text.chars().collect();
    let mut start = pos.min(chars.len().saturating_sub(1));

    // If on whitespace, move back to find a word
    while start > 0 && !is_word_char(chars[start]) {
        start -= 1;
    }

    // Find the start of this word
    while start > 0 && is_word_char(chars[start - 1]) {
        start -= 1;
    }
    start
}

/// Find the end of a word at the given position
pub fn find_word_end(text: &str, pos: usize) -> usize {
    if text.is_empty() {
        return 0;
    }
    let chars: Vec<char> = text.chars().collect();
    let mut end = pos.min(chars.len().saturating_sub(1));

    // If on whitespace, move forward to find a word
    while end < chars.len() && !is_word_char(chars[end]) {
        end += 1;
    }

    // Find the end of this word
    while end < chars.len() && is_word_char(chars[end]) {
        end += 1;
    }
    end
}

/// Configuration for TextInput appearance
#[derive(Clone)]
pub struct TextInputStyle {
    pub height: f32,
    pub font_size: f32,
    pub padding_x: f32,
    pub border_radius: f32,
    pub bg_color: Option<Hsla>,
    pub border_color: Option<Hsla>,
    pub border_focused_color: Option<Hsla>,
    pub text_color: Option<Hsla>,
    pub placeholder_color: Option<Hsla>,
}

impl Default for TextInputStyle {
    fn default() -> Self {
        Self {
            height: 32.0,
            font_size: 13.0,
            padding_x: 12.0,
            border_radius: 4.0,
            bg_color: None,
            border_color: None,
            border_focused_color: None,
            text_color: None,
            placeholder_color: None,
        }
    }
}

impl TextInputStyle {
    pub fn small() -> Self {
        Self {
            height: 24.0,
            font_size: 11.0,
            padding_x: 6.0,
            border_radius: 4.0,
            ..Default::default()
        }
    }

    pub fn compact() -> Self {
        Self {
            height: 28.0,
            font_size: 12.0,
            padding_x: 8.0,
            border_radius: 4.0,
            ..Default::default()
        }
    }
}

/// Snapshot of text state for undo/redo
#[derive(Clone)]
struct TextState {
    text: String,
    selection: Range<usize>,
}

/// A full-featured text input component
pub struct TextInput {
    /// Unique ID for this input
    id: SharedString,
    /// The text content
    text: String,
    /// Selection range (start..end), cursor is at end
    selection: Range<usize>,
    /// Focus handle
    focus: FocusHandle,
    /// Placeholder text
    placeholder: SharedString,
    /// Whether mouse is currently selecting
    is_selecting: bool,
    /// Input element left offset for click calculation
    input_left: f32,
    /// Styling configuration
    style: TextInputStyle,
    /// Whether to allow multiline (for body editors)
    multiline: bool,
    /// Undo stack
    undo_stack: Vec<TextState>,
    /// Redo stack
    redo_stack: Vec<TextState>,
    /// Callback when text changes
    on_change: Option<Box<dyn Fn(&str, &mut Context<Self>) + 'static>>,
    /// Callback when Enter is pressed (single-line mode)
    on_submit: Option<Box<dyn Fn(&str, &mut Context<Self>) + 'static>>,
    /// Callback when focus is lost
    on_blur: Option<Box<dyn Fn(&str, &mut Context<Self>) + 'static>>,
}

impl TextInput {
    pub fn new(cx: &mut Context<Self>, id: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            text: String::new(),
            selection: 0..0,
            focus: cx.focus_handle(),
            placeholder: SharedString::default(),
            is_selecting: false,
            input_left: 0.0,
            style: TextInputStyle::default(),
            multiline: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            on_change: None,
            on_submit: None,
            on_blur: None,
        }
    }

    // Builder methods
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = text.into();
        let len = self.text.len();
        self.selection = len..len;
        self
    }

    pub fn style(mut self, style: TextInputStyle) -> Self {
        self.style = style;
        self
    }

    pub fn multiline(mut self, multiline: bool) -> Self {
        self.multiline = multiline;
        self
    }

    pub fn on_change(mut self, callback: impl Fn(&str, &mut Context<Self>) + 'static) -> Self {
        self.on_change = Some(Box::new(callback));
        self
    }

    pub fn on_submit(mut self, callback: impl Fn(&str, &mut Context<Self>) + 'static) -> Self {
        self.on_submit = Some(Box::new(callback));
        self
    }

    pub fn on_blur(mut self, callback: impl Fn(&str, &mut Context<Self>) + 'static) -> Self {
        self.on_blur = Some(Box::new(callback));
        self
    }

    // Public API
    pub fn get_text(&self) -> &str {
        &self.text
    }

    pub fn set_text(&mut self, text: impl Into<String>, cx: &mut Context<Self>) {
        self.text = text.into();
        let len = self.text.len();
        self.selection = len..len;
        cx.notify();
    }

    pub fn focus(&self, window: &mut Window, cx: &mut Context<Self>) {
        self.focus.focus(window, cx);
    }

    pub fn is_focused(&self, window: &Window) -> bool {
        self.focus.is_focused(window)
    }

    pub fn select_all(&mut self, cx: &mut Context<Self>) {
        self.selection = 0..self.text.len();
        cx.notify();
    }

    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.text.clear();
        self.selection = 0..0;
        if let Some(on_change) = &self.on_change {
            on_change(&self.text, cx);
        }
        cx.notify();
    }

    /// Save current state to undo stack before making changes
    fn save_state(&mut self) {
        let state = TextState {
            text: self.text.clone(),
            selection: self.selection.clone(),
        };
        self.undo_stack.push(state);
        // Limit undo history size
        if self.undo_stack.len() > MAX_UNDO_HISTORY {
            self.undo_stack.remove(0);
        }
        // Clear redo stack on new action
        self.redo_stack.clear();
    }

    /// Undo the last change
    fn undo(&mut self, cx: &mut Context<Self>) {
        if let Some(state) = self.undo_stack.pop() {
            // Save current state to redo stack
            let current = TextState {
                text: self.text.clone(),
                selection: self.selection.clone(),
            };
            self.redo_stack.push(current);

            // Restore previous state
            self.text = state.text;
            self.selection = state.selection;

            if let Some(on_change) = &self.on_change {
                on_change(&self.text, cx);
            }
            cx.notify();
        }
    }

    /// Redo the last undone change
    fn redo(&mut self, cx: &mut Context<Self>) {
        if let Some(state) = self.redo_stack.pop() {
            // Save current state to undo stack
            let current = TextState {
                text: self.text.clone(),
                selection: self.selection.clone(),
            };
            self.undo_stack.push(current);

            // Restore redo state
            self.text = state.text;
            self.selection = state.selection;

            if let Some(on_change) = &self.on_change {
                on_change(&self.text, cx);
            }
            cx.notify();
        }
    }

    // Internal helpers
    fn cursor(&self) -> usize {
        self.selection.end
    }

    fn has_selection(&self) -> bool {
        self.selection.start != self.selection.end
    }

    fn selected_text(&self) -> &str {
        let start = self.selection.start.min(self.selection.end);
        let end = self.selection.start.max(self.selection.end);
        &self.text[start..end]
    }

    fn move_to(&mut self, pos: usize, cx: &mut Context<Self>) {
        let pos = pos.min(self.text.len());
        self.selection = pos..pos;
        cx.notify();
    }

    fn select_to(&mut self, pos: usize, cx: &mut Context<Self>) {
        let pos = pos.min(self.text.len());
        self.selection.end = pos;
        cx.notify();
    }

    fn delete_selection(&mut self, cx: &mut Context<Self>) {
        if self.has_selection() {
            self.save_state();
            let start = self.selection.start.min(self.selection.end);
            let end = self.selection.start.max(self.selection.end);
            self.text.replace_range(start..end, "");
            self.selection = start..start;
            if let Some(on_change) = &self.on_change {
                on_change(&self.text, cx);
            }
            cx.notify();
        }
    }

    fn insert_text(&mut self, insert: &str, cx: &mut Context<Self>) {
        self.save_state();
        // Delete selection first
        if self.has_selection() {
            let start = self.selection.start.min(self.selection.end);
            let end = self.selection.start.max(self.selection.end);
            self.text.replace_range(start..end, "");
            self.selection = start..start;
        }

        let pos = self.selection.start;
        self.text.insert_str(pos, insert);
        let new_pos = pos + insert.len();
        self.selection = new_pos..new_pos;

        if let Some(on_change) = &self.on_change {
            on_change(&self.text, cx);
        }
        cx.notify();
    }

    fn char_width(&self) -> f32 {
        self.style.font_size * 0.6 // Approximate monospace width
    }

    fn index_for_x(&self, x: f32) -> usize {
        if x <= 0.0 {
            0
        } else {
            let approx_char = (x / self.char_width()) as usize;
            approx_char.min(self.text.len())
        }
    }

    fn handle_mouse_down(&mut self, event: &MouseDownEvent, cx: &mut Context<Self>) {
        self.is_selecting = true;
        let click_x = f32::from(event.position.x) - self.input_left;
        let index = self.index_for_x(click_x);

        // Cycle: 1=cursor, 2=word, 3=all, 4+=cursor
        let effective_click = if event.click_count >= 4 { 1 } else { event.click_count };

        match effective_click {
            2 => {
                // Double-click: select word
                let start = find_word_start(&self.text, index);
                let end = find_word_end(&self.text, index);
                self.selection = start..end;
                cx.notify();
            }
            3 => {
                // Triple-click: select all
                self.select_all(cx);
            }
            _ => {
                // Single click (or 4th+ to deselect)
                if event.modifiers.shift {
                    self.select_to(index, cx);
                } else {
                    self.move_to(index, cx);
                }
            }
        }
    }

    fn handle_mouse_move(&mut self, event: &MouseMoveEvent, cx: &mut Context<Self>) {
        if self.is_selecting {
            let click_x = f32::from(event.position.x) - self.input_left;
            let index = self.index_for_x(click_x);
            self.selection.end = index.min(self.text.len());
            cx.notify();
        }
    }

    fn handle_mouse_up(&mut self, _event: &MouseUpEvent, _cx: &mut Context<Self>) {
        self.is_selecting = false;
    }

    fn handle_key(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let key = event.keystroke.key.as_str();
        let ctrl = event.keystroke.modifiers.control;
        let shift = event.keystroke.modifiers.shift;

        // Handle Ctrl shortcuts
        if ctrl {
            match key {
                "a" => {
                    self.select_all(cx);
                    return;
                }
                "c" => {
                    if self.has_selection() {
                        cx.write_to_clipboard(ClipboardItem::new_string(
                            self.selected_text().to_string(),
                        ));
                    }
                    return;
                }
                "x" => {
                    if self.has_selection() {
                        cx.write_to_clipboard(ClipboardItem::new_string(
                            self.selected_text().to_string(),
                        ));
                        self.delete_selection(cx);
                    }
                    return;
                }
                "v" => {
                    if let Some(item) = cx.read_from_clipboard() {
                        if let Some(text) = item.text() {
                            let insert_text = if self.multiline {
                                text.to_string()
                            } else {
                                text.replace('\n', "")
                            };
                            self.insert_text(&insert_text, cx);
                        }
                    }
                    return;
                }
                "z" => {
                    if shift {
                        // Ctrl+Shift+Z = Redo
                        self.redo(cx);
                    } else {
                        // Ctrl+Z = Undo
                        self.undo(cx);
                    }
                    return;
                }
                "y" => {
                    // Ctrl+Y = Redo (alternative)
                    self.redo(cx);
                    return;
                }
                _ => {}
            }
        }

        match key {
            "left" => {
                if shift {
                    if self.selection.end > 0 {
                        self.selection.end -= 1;
                        cx.notify();
                    }
                } else if self.has_selection() {
                    let start = self.selection.start.min(self.selection.end);
                    self.move_to(start, cx);
                } else if self.cursor() > 0 {
                    self.move_to(self.cursor() - 1, cx);
                }
            }
            "right" => {
                if shift {
                    if self.selection.end < self.text.len() {
                        self.selection.end += 1;
                        cx.notify();
                    }
                } else if self.has_selection() {
                    let end = self.selection.start.max(self.selection.end);
                    self.move_to(end, cx);
                } else if self.cursor() < self.text.len() {
                    self.move_to(self.cursor() + 1, cx);
                }
            }
            "home" => {
                if shift {
                    self.selection.end = 0;
                    cx.notify();
                } else {
                    self.move_to(0, cx);
                }
            }
            "end" => {
                if shift {
                    self.selection.end = self.text.len();
                    cx.notify();
                } else {
                    self.move_to(self.text.len(), cx);
                }
            }
            "backspace" => {
                if self.has_selection() {
                    self.delete_selection(cx);
                } else if self.cursor() > 0 {
                    self.save_state();
                    let pos = self.cursor() - 1;
                    self.text.remove(pos);
                    self.selection = pos..pos;
                    if let Some(on_change) = &self.on_change {
                        on_change(&self.text, cx);
                    }
                    cx.notify();
                }
            }
            "delete" => {
                if self.has_selection() {
                    self.delete_selection(cx);
                } else if self.cursor() < self.text.len() {
                    self.save_state();
                    self.text.remove(self.cursor());
                    if let Some(on_change) = &self.on_change {
                        on_change(&self.text, cx);
                    }
                    cx.notify();
                }
            }
            "enter" => {
                if self.multiline {
                    self.insert_text("\n", cx);
                } else if let Some(on_submit) = &self.on_submit {
                    on_submit(&self.text, cx);
                }
            }
            "escape" => {
                // Could trigger blur or cancel
            }
            "tab" => {
                // Let parent handle tab navigation
            }
            _ => {
                // Handle printable characters
                if let Some(ch) = &event.keystroke.key_char {
                    self.insert_text(ch, cx);
                }
            }
        }
    }

    fn render_text_with_selection(&self, is_focused: bool, cx: &Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let text_color = self.style.text_color.unwrap_or(theme.colors.text_primary);
        let char_width = self.char_width();
        let font_size = self.style.font_size;

        if self.text.is_empty() {
            // Empty text - show cursor if focused
            return div()
                .flex()
                .items_center()
                .h_full()
                .relative()
                .when(is_focused, |el| {
                    el.child(
                        div()
                            .w(px(2.0))
                            .h(px(font_size + 2.0))
                            .bg(text_color),
                    )
                })
                .into_any_element();
        }

        let sel_start = self.selection.start.min(self.selection.end).min(self.text.len());
        let sel_end = self.selection.start.max(self.selection.end).min(self.text.len());
        let has_sel = sel_start != sel_end;
        let cursor_pos = self.selection.end.min(self.text.len());

        // Render each character in a fixed-width container for consistent positioning
        div()
            .flex()
            .items_center()
            .h_full()
            .relative()
            // Selection highlight (absolute positioned)
            .when(has_sel, |el| {
                let sel_x = sel_start as f32 * char_width;
                let sel_width = (sel_end - sel_start) as f32 * char_width;
                el.child(
                    div()
                        .absolute()
                        .top_0()
                        .bottom_0()
                        .left(px(sel_x))
                        .w(px(sel_width))
                        .bg(gpui::rgba(0x3366ff40))
                )
            })
            // Cursor (absolute positioned, centered vertically)
            .when(is_focused, |el| {
                let cursor_x = cursor_pos as f32 * char_width;
                el.child(
                    div()
                        .absolute()
                        .top_1()
                        .bottom_1()
                        .left(px(cursor_x))
                        .w(px(2.0))
                        .bg(text_color)
                )
            })
            // Characters in fixed-width containers
            .child(
                div()
                    .flex()
                    .text_size(px(font_size))
                    .font_family("Ubuntu Mono")
                    .text_color(text_color)
                    .children(self.text.chars().map(|c| {
                        div()
                            .w(px(char_width))
                            .child(c.to_string())
                    }))
            )
            .into_any_element()
    }
}

impl Render for TextInput {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let is_focused = self.focus.is_focused(window);
        let placeholder = self.placeholder.clone();

        let bg_color = self.style.bg_color.unwrap_or(theme.colors.bg_tertiary);
        let border_color = if is_focused {
            self.style.border_focused_color.unwrap_or(theme.colors.border_focused)
        } else {
            self.style.border_color.unwrap_or(theme.colors.border)
        };
        let placeholder_color = self.style.placeholder_color.unwrap_or(theme.colors.text_muted);

        // Store input_left for click calculations
        let padding = self.style.padding_x;

        div()
            .id(self.id.clone())
            .h(px(self.style.height))
            .w_full()
            .min_w(px(0.0))
            .px(px(padding))
            .flex()
            .items_center()
            .overflow_hidden()
            .rounded(px(self.style.border_radius))
            .border_1()
            .border_color(border_color)
            .bg(bg_color)
            .cursor_text()
            .track_focus(&self.focus)
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                    this.input_left = padding + f32::from(event.position.x) - f32::from(event.position.x);
                    // Approximate input_left from event - this is tricky without layout info
                    // For now, use padding as approximation
                    this.input_left = padding;
                    this.focus.focus(window, cx);
                    this.handle_mouse_down(event, cx);
                }),
            )
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                this.handle_mouse_move(event, cx);
            }))
            .on_mouse_up(
                gpui::MouseButton::Left,
                cx.listener(|this, event: &MouseUpEvent, _, cx| {
                    this.handle_mouse_up(event, cx);
                }),
            )
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                this.handle_key(event, cx);
            }))
            .child(
                div()
                    .flex()
                    .items_center()
                    .text_size(px(self.style.font_size))
                    .font_family("Ubuntu Mono")
                    .child(if self.text.is_empty() && !is_focused {
                        div()
                            .text_color(placeholder_color)
                            .child(placeholder)
                            .into_any_element()
                    } else {
                        self.render_text_with_selection(is_focused, cx)
                    }),
            )
    }
}

// ============================================================================
// Standalone helper functions for use by parent components
// ============================================================================

/// Render text with selection highlighting and cursor
///
/// Use this for inline text rendering in parent components that manage their own state.
/// Returns an AnyElement that can be composed into parent UI.
///
/// `max_chars` - optional max characters before truncation (only when unfocused)
pub fn render_text_view(
    text: &str,
    selection: &std::ops::Range<usize>,
    is_focused: bool,
    font_size: f32,
    text_color: Hsla,
    placeholder: Option<&str>,
    placeholder_color: Hsla,
) -> gpui::AnyElement {
    render_text_view_with_max(text, selection, is_focused, font_size, text_color, placeholder, placeholder_color, None)
}

/// Render text with optional max character limit for truncation
/// When focused, expands to multiple lines if chars_per_line is provided
pub fn render_text_view_with_max(
    text: &str,
    selection: &std::ops::Range<usize>,
    is_focused: bool,
    font_size: f32,
    text_color: Hsla,
    placeholder: Option<&str>,
    placeholder_color: Hsla,
    max_chars: Option<usize>,
) -> gpui::AnyElement {
    // Use default chars_per_line based on max_chars for multi-line when focused
    let chars_per_line = max_chars.map(|m| m.max(10));
    render_text_view_multiline(
        text,
        selection,
        is_focused,
        font_size,
        text_color,
        placeholder,
        placeholder_color,
        max_chars,
        chars_per_line,
    )
}

/// Render text with multi-line support when focused
/// - `max_chars`: truncation limit when unfocused
/// - `chars_per_line`: characters per line when focused (enables multi-line wrapping)
pub fn render_text_view_multiline(
    text: &str,
    selection: &std::ops::Range<usize>,
    is_focused: bool,
    font_size: f32,
    text_color: Hsla,
    placeholder: Option<&str>,
    placeholder_color: Hsla,
    max_chars: Option<usize>,
    chars_per_line: Option<usize>,
) -> gpui::AnyElement {
    use gpui::IntoElement;

    if text.is_empty() {
        if let Some(ph) = placeholder {
            if !is_focused {
                return div()
                    .flex()
                    .items_center()
                    .text_size(px(font_size))
                    .text_color(placeholder_color)
                    .child(ph.to_string())
                    .into_any_element();
            }
        }
        // Empty but focused - show cursor
        return div()
            .flex()
            .items_center()
            .text_size(px(font_size))
            .text_color(text_color)
            .when(is_focused, |el| {
                el.child(
                    div()
                        .w(px(1.0))
                        .h(px(font_size + 2.0))
                        .bg(text_color),
                )
            })
            .into_any_element();
    }

    // Unfocused: show truncated text with ellipsis (single line)
    if !is_focused {
        let display_text = if let Some(max) = max_chars {
            if text.chars().count() > max {
                let truncated: String = text.chars().take(max.saturating_sub(1)).collect();
                format!("{}…", truncated)
            } else {
                text.to_string()
            }
        } else {
            text.to_string()
        };

        return div()
            .flex()
            .items_center()
            .text_size(px(font_size))
            .text_color(text_color)
            .child(display_text)
            .into_any_element();
    }

    // Focused: show full text with multi-line wrapping
    let sel_start = selection.start.min(selection.end).min(text.len());
    let sel_end = selection.start.max(selection.end).min(text.len());
    let has_sel = sel_start != sel_end;
    let cursor_pos = sel_end;

    // Character width for fixed-width rendering
    let char_width = font_size * 0.6;

    // If no chars_per_line specified, render single line with fixed-width chars
    let cpl = match chars_per_line {
        Some(c) if c > 0 => c,
        _ => {
            // Single line with fixed-width character rendering
            return div()
                .flex()
                .items_center()
                .h_full()
                .relative()
                // Selection highlight (absolute positioned)
                .when(has_sel, |el| {
                    let sel_x = sel_start as f32 * char_width;
                    let sel_width = (sel_end - sel_start) as f32 * char_width;
                    el.child(
                        div()
                            .absolute()
                            .top_0()
                            .bottom_0()
                            .left(px(sel_x))
                            .w(px(sel_width))
                            .bg(gpui::rgba(0x3366ff40))
                    )
                })
                // Cursor (absolute positioned, centered vertically)
                .child(
                    div()
                        .absolute()
                        .top_1()
                        .bottom_1()
                        .left(px(cursor_pos as f32 * char_width))
                        .w(px(2.0))
                        .bg(text_color)
                )
                // Characters in fixed-width containers (no monospace font)
                .child(
                    div()
                        .flex()
                        .text_size(px(font_size))
                        .text_color(text_color)
                        .children(text.chars().map(|c| {
                            div()
                                .w(px(char_width))
                                .flex()
                                .justify_center()
                                .child(c.to_string())
                        }))
                )
                .into_any_element();
        }
    };

    // Break text into lines
    let chars: Vec<char> = text.chars().collect();
    let mut lines: Vec<String> = Vec::new();
    let mut current_line = String::new();

    for ch in &chars {
        current_line.push(*ch);
        if current_line.chars().count() >= cpl {
            lines.push(current_line);
            current_line = String::new();
        }
    }
    if !current_line.is_empty() {
        lines.push(current_line);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }

    // Build multi-line display with fixed-width character rendering
    let line_height = font_size + 4.0;

    div()
        .flex()
        .flex_col()
        .w_full()
        .text_size(px(font_size))
        .text_color(text_color)
        .children(lines.iter().enumerate().map(|(line_idx, line_text)| {
            let line_start = line_idx * cpl;
            let line_end = line_start + line_text.chars().count();

            // Check if cursor/selection is on this line
            let cursor_on_line = !has_sel && cursor_pos >= line_start && cursor_pos <= line_end;
            let sel_intersects = has_sel && sel_start < line_end && sel_end > line_start;

            // Calculate local positions
            let local_sel_start = sel_start.saturating_sub(line_start).min(line_text.chars().count());
            let local_sel_end = sel_end.saturating_sub(line_start).min(line_text.chars().count());
            let local_cursor = cursor_pos.saturating_sub(line_start).min(line_text.chars().count());

            div()
                .h(px(line_height))
                .flex()
                .items_center()
                .relative()
                // Selection highlight (absolute positioned)
                .when(sel_intersects, |el| {
                    let sel_x = local_sel_start as f32 * char_width;
                    let sel_width = (local_sel_end - local_sel_start) as f32 * char_width;
                    el.child(
                        div()
                            .absolute()
                            .top_0()
                            .bottom_0()
                            .left(px(sel_x))
                            .w(px(sel_width.max(2.0)))
                            .bg(gpui::rgba(0x3366ff40))
                    )
                })
                // Cursor (absolute positioned)
                .when(cursor_on_line || (sel_intersects && sel_end >= line_start && sel_end <= line_end), |el| {
                    let cursor_x = if cursor_on_line { local_cursor } else { local_sel_end };
                    el.child(
                        div()
                            .absolute()
                            .top(px(2.0))
                            .left(px(cursor_x as f32 * char_width))
                            .w(px(2.0))
                            .h(px(font_size + 2.0))
                            .bg(text_color)
                    )
                })
                // Characters in fixed-width containers
                .child(
                    div()
                        .flex()
                        .children(line_text.chars().map(|c| {
                            div()
                                .w(px(char_width))
                                .child(c.to_string())
                        }))
                )
        }))
        .into_any_element()
}

/// Calculate character index from x position
pub fn index_for_x(x: f32, text_len: usize, char_width: f32) -> usize {
    if x <= 0.0 {
        0
    } else {
        let approx_char = (x / char_width) as usize;
        approx_char.min(text_len)
    }
}

/// Handle click count for selection cycling: 1=cursor, 2=word, 3=all, 4+=cursor
pub fn effective_click_count(click_count: usize) -> usize {
    if click_count >= 4 { 1 } else { click_count }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_word_char() {
        assert!(is_word_char('a'));
        assert!(is_word_char('Z'));
        assert!(is_word_char('5'));
        assert!(is_word_char('_'));
        assert!(!is_word_char(' '));
        assert!(!is_word_char('.'));
        assert!(!is_word_char('/'));
    }

    #[test]
    fn test_find_word_start() {
        let text = "hello world";
        assert_eq!(find_word_start(text, 0), 0);
        assert_eq!(find_word_start(text, 3), 0);
        assert_eq!(find_word_start(text, 8), 6);
    }

    #[test]
    fn test_find_word_end() {
        let text = "hello world";
        assert_eq!(find_word_end(text, 0), 5);
        assert_eq!(find_word_end(text, 3), 5);
        assert_eq!(find_word_end(text, 8), 11);
    }

    #[test]
    fn test_find_word_boundaries_empty() {
        assert_eq!(find_word_start("", 0), 0);
        assert_eq!(find_word_end("", 0), 0);
    }

    #[test]
    fn test_text_input_style_default() {
        let style = TextInputStyle::default();
        assert_eq!(style.height, 32.0);
        assert_eq!(style.font_size, 13.0);
    }

    #[test]
    fn test_text_input_style_small() {
        let style = TextInputStyle::small();
        assert_eq!(style.height, 24.0);
        assert_eq!(style.font_size, 11.0);
    }
}
