//! Reusable text input component with full editing support
//!
//! Features:
//! - Single/double/triple/quad click selection
//! - Mouse drag selection
//! - Keyboard navigation (arrows, home, end)
//! - Selection extension (Shift+Arrow)
//! - Copy/Cut/Paste (Ctrl+C/X/V)
//! - Select All (Ctrl+A)
//! - Customizable styling

#![allow(dead_code)]

use std::ops::Range;

use gpui::{
    div, prelude::*, px, ClipboardItem, Context, FocusHandle, Hsla, IntoElement,
    KeyDownEvent, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement,
    Render, SharedString, Styled, Window,
};

use crate::theme;

/// Check if character is a word character (alphanumeric or underscore)
fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Find the start of a word at the given position
fn find_word_start(text: &str, pos: usize) -> usize {
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
fn find_word_end(text: &str, pos: usize) -> usize {
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

        if self.text.is_empty() {
            return div()
                .flex()
                .items_center()
                .text_color(text_color)
                .into_any_element();
        }

        let sel_start = self.selection.start.min(self.selection.end).min(self.text.len());
        let sel_end = self.selection.start.max(self.selection.end).min(self.text.len());
        let has_sel = sel_start != sel_end;

        let before = &self.text[..sel_start];
        let selected = &self.text[sel_start..sel_end];
        let after = &self.text[sel_end..];

        div()
            .flex()
            .items_center()
            .text_color(text_color)
            .child(before.to_string())
            .when(has_sel, |el| {
                el.child(
                    div()
                        .bg(gpui::rgba(0x3366ff40))
                        .child(selected.to_string()),
                )
            })
            .when(!has_sel && is_focused, |el| {
                el.child(
                    div()
                        .w(px(1.0))
                        .h(px(self.style.font_size + 2.0))
                        .bg(text_color),
                )
            })
            .child(after.to_string())
            .when(has_sel && is_focused, |el| {
                el.child(
                    div()
                        .w(px(1.0))
                        .h(px(self.style.font_size + 2.0))
                        .bg(text_color),
                )
            })
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
            .px(px(padding))
            .flex()
            .items_center()
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
                    .font_family("monospace")
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
