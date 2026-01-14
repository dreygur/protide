//! Reusable text input component with cursor support

#![allow(dead_code)]

use gpui::{
    div, prelude::*, px, App, Context, FocusHandle, IntoElement, KeyDownEvent,
    MouseDownEvent, ParentElement, Render, SharedString, Styled, Window,
};

use crate::theme;

/// A text input component with cursor positioning
pub struct TextInput {
    /// The text content
    text: String,
    /// Cursor position (character index)
    cursor: usize,
    /// Focus handle
    focus: FocusHandle,
    /// Placeholder text
    placeholder: SharedString,
    /// Callback when text changes
    on_change: Option<Box<dyn Fn(&str, &mut Window, &mut App) + 'static>>,
    /// Callback when Enter is pressed
    on_submit: Option<Box<dyn Fn(&str, &mut Window, &mut App) + 'static>>,
}

impl TextInput {
    pub fn new(cx: &mut Context<Self>, placeholder: impl Into<SharedString>) -> Self {
        Self {
            text: String::new(),
            cursor: 0,
            focus: cx.focus_handle(),
            placeholder: placeholder.into(),
            on_change: None,
            on_submit: None,
        }
    }

    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text = text.into();
        self.cursor = self.text.len();
        self
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn set_text(&mut self, text: impl Into<String>, cx: &mut Context<Self>) {
        self.text = text.into();
        self.cursor = self.cursor.min(self.text.len());
        cx.notify();
    }

    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        self.focus.focus(window, cx);
    }

    fn handle_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = event.keystroke.key.as_str();

        match key {
            "left" => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                    cx.notify();
                }
            }
            "right" => {
                if self.cursor < self.text.len() {
                    self.cursor += 1;
                    cx.notify();
                }
            }
            "home" => {
                self.cursor = 0;
                cx.notify();
            }
            "end" => {
                self.cursor = self.text.len();
                cx.notify();
            }
            "backspace" => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                    self.text.remove(self.cursor);
                    cx.notify();
                    if let Some(on_change) = &self.on_change {
                        on_change(&self.text, window, cx);
                    }
                }
            }
            "delete" => {
                if self.cursor < self.text.len() {
                    self.text.remove(self.cursor);
                    cx.notify();
                    if let Some(on_change) = &self.on_change {
                        on_change(&self.text, window, cx);
                    }
                }
            }
            "enter" => {
                if let Some(on_submit) = &self.on_submit {
                    on_submit(&self.text, window, cx);
                }
            }
            _ => {
                // Handle printable characters
                if let Some(ch) = &event.keystroke.key_char {
                    self.text.insert_str(self.cursor, ch);
                    self.cursor += ch.len();
                    cx.notify();
                    if let Some(on_change) = &self.on_change {
                        on_change(&self.text, window, cx);
                    }
                }
            }
        }
    }

    fn handle_click(&mut self, event: &MouseDownEvent, cx: &mut Context<Self>) {
        // Approximate character position from click x coordinate
        // Assuming ~8px per character (monospace approximation)
        let char_width: f32 = 8.0;
        let padding: f32 = 12.0;
        let click_x = f32::from(event.position.x) - padding;

        if click_x <= 0.0 {
            self.cursor = 0;
        } else {
            let approx_char = (click_x / char_width) as usize;
            self.cursor = approx_char.min(self.text.len());
        }
        cx.notify();
    }
}

impl Render for TextInput {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let is_focused = self.focus.is_focused(window);
        let text = self.text.clone();
        let _cursor = self.cursor;
        let placeholder = self.placeholder.clone();

        div()
            .id("text-input")
            .h(px(32.0))
            .w_full()
            .px(px(12.0))
            .flex()
            .items_center()
            .rounded(px(4.0))
            .border_1()
            .when(is_focused, |el| el.border_color(theme.colors.border_focused))
            .when(!is_focused, |el| el.border_color(theme.colors.border))
            .bg(theme.colors.bg_tertiary)
            .cursor_text()
            .track_focus(&self.focus)
            .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, event: &MouseDownEvent, window, cx| {
                this.focus.focus(window, cx);
                this.handle_click(event, cx);
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                this.handle_key(event, window, cx);
            }))
            .child(
                div()
                    .flex()
                    .items_center()
                    .text_size(px(13.0))
                    .child(if text.is_empty() && !is_focused {
                        // Show placeholder
                        div()
                            .text_color(theme.colors.text_muted)
                            .child(placeholder)
                    } else {
                        // Show text with cursor
                        div()
                            .flex()
                            .items_center()
                            .text_color(theme.colors.text_primary)
                            .child(self.render_text_with_cursor(is_focused, cx))
                    })
            )
    }
}

impl TextInput {
    fn render_text_with_cursor(&self, show_cursor: bool, cx: &Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let before_cursor = &self.text[..self.cursor];
        let after_cursor = &self.text[self.cursor..];

        div()
            .flex()
            .items_center()
            .child(
                div().child(before_cursor.to_string())
            )
            .when(show_cursor, |el| {
                el.child(
                    div()
                        .w(px(1.0))
                        .h(px(16.0))
                        .bg(theme.colors.text_primary)
                )
            })
            .child(
                div().child(after_cursor.to_string())
            )
    }
}
