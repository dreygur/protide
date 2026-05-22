#![allow(dead_code)]

use gpui::{div, font, prelude::*, px, Hsla, StyledText, TextRun};

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

/// Render text with selection highlighting and cursor
///
/// Render text with optional max character limit for truncation
/// When focused, expands to multiple lines if chars_per_line is provided
/// `scroll_offset_x`: horizontal pixel scroll offset (0.0 = no scroll)
pub fn render_text_view_with_max(
    text: &str,
    selection: &std::ops::Range<usize>,
    is_focused: bool,
    font_size: f32,
    text_color: Hsla,
    placeholder: Option<&str>,
    placeholder_color: Hsla,
    max_chars: Option<usize>,
    selection_bg: Hsla,
) -> gpui::AnyElement {
    render_text_view_with_max_scrolled(text, selection, is_focused, font_size, text_color, placeholder, placeholder_color, max_chars, selection_bg, 0.0)
}

pub fn render_text_view_with_max_scrolled(
    text: &str,
    selection: &std::ops::Range<usize>,
    is_focused: bool,
    font_size: f32,
    text_color: Hsla,
    placeholder: Option<&str>,
    placeholder_color: Hsla,
    max_chars: Option<usize>,
    selection_bg: Hsla,
    scroll_offset_x: f32,
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
        selection_bg,
        scroll_offset_x,
    )
}

/// Render text with multi-line support when focused
/// - `max_chars`: truncation limit when unfocused
/// - `chars_per_line`: characters per line when focused (enables multi-line wrapping)
/// - `scroll_offset_x`: horizontal pixel offset for single-line scroll (0.0 = no scroll)
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
    selection_bg: Hsla,
    scroll_offset_x: f32,
) -> gpui::AnyElement {
    use gpui::IntoElement;

    if text.is_empty() {
        if let Some(ph) = placeholder
            && !is_focused {
                return div()
                    .flex()
                    .items_center()
                    .text_size(px(font_size))
                    .font_family("JetBrains Mono")
                    .text_color(placeholder_color)
                    .child(ph.to_string())
                    .into_any_element();
            }
        // Empty but focused - show cursor
        return div()
            .flex()
            .items_center()
            .text_size(px(font_size))
            .font_family("JetBrains Mono")
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
            .font_family("JetBrains Mono")
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
                .left(px(-scroll_offset_x))
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
                            .bg(selection_bg)
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
                .child(
                    div()
                        .text_size(px(font_size))
                        .child(StyledText::new(text.to_string()).with_runs(vec![TextRun {
                            len: text.len(),
                            font: font("JetBrains Mono"),
                            color: text_color,
                            background_color: None,
                            underline: None,
                            strikethrough: None,
                        }]))
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
        .font_family("JetBrains Mono")
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
                            .bg(selection_bg)
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
                .child(
                    div()
                        .text_size(px(font_size))
                        .child(StyledText::new(line_text.clone()).with_runs(vec![TextRun {
                            len: line_text.len(),
                            font: font("JetBrains Mono"),
                            color: text_color,
                            background_color: None,
                            underline: None,
                            strikethrough: None,
                        }]))
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
