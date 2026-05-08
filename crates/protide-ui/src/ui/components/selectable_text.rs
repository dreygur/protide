use gpui::{
    div, prelude::*, px, Context, ElementId, Hsla, InteractiveElement,
    IntoElement, ParentElement, SharedString, Styled, Window,
};

/// A selection range spanning multiple rows.
/// Tracks start/end row indices and byte offsets within each row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SelectionRange {
    pub start_row: usize,
    pub start_offset: usize,
    pub end_row: usize,
    pub end_offset: usize,
}

impl SelectionRange {
    pub fn new(start_row: usize, start_offset: usize, end_row: usize, end_offset: usize) -> Self {
        Self { start_row, start_offset, end_row, end_offset }
    }

    /// Check if a given (row_index, offset) falls within this selection.
    pub fn contains(&self, row: usize, offset: usize) -> bool {
        if self.start_row == self.end_row {
            row == self.start_row
                && offset >= self.start_offset.min(self.end_offset)
                && offset < self.start_offset.max(self.end_offset)
        } else if row == self.start_row {
            offset >= self.start_offset.min(self.end_offset)
        } else if row == self.end_row {
            offset < self.end_offset.max(self.start_offset)
        } else {
            row > self.start_row.min(self.end_row) && row < self.end_row.max(self.start_row)
        }
    }

    /// Returns the (start, end) byte offsets for a given row, if the row intersects selection.
    pub fn offsets_for_row(&self, row: usize, text_len: usize) -> Option<(usize, usize)> {
        let (sr, er) = if self.start_row <= self.end_row {
            (self.start_row, self.end_row)
        } else {
            (self.end_row, self.start_row)
        };
        let (so, eo) = if sr == self.start_row {
            (self.start_offset, self.end_offset)
        } else {
            (self.end_offset, self.start_offset)
        };

        if row < sr || row > er {
            return None;
        }
        if sr == er {
            let s = so.min(eo).min(text_len);
            let e = eo.max(so).min(text_len);
            return Some((s, e));
        }
        if row == sr {
            let s = so.min(eo).min(text_len);
            Some((s, text_len))
        } else if row == er {
            let e = eo.max(so).min(text_len);
            Some((0, e))
        } else {
            Some((0, text_len))
        }
    }
}

/// A text element that supports mouse-driven selection and Ctrl+C copy.
///
/// Architecture: Uses a global logical selection layer. Tracks `selection` as
/// `(usize, usize)` byte offsets into the full text. Renders as three segments:
/// before-selection, selection-highlighted, after-selection.
///
/// For multi-row support, pair with `SelectionRange` at the parent level.
pub struct SelectableText {
    /// Unique element ID for event routing
    id: ElementId,
    /// Full text content
    text: SharedString,
    /// Current selection as byte offsets `(start, end)` — `None` means no selection
    selection: Option<(usize, usize)>,
    /// Whether we're in the middle of a mouse-drag selection
    selecting: bool,
    /// The base color for non-selected text
    text_color: Hsla,
    /// The highlight color for the selection
    selection_color: Hsla,
    /// Monospace style
    mono: bool,
    /// Font size in pixels
    font_size: f32,
    /// Row index for multi-row selection tracking
    row_index: Option<usize>,
}

impl SelectableText {
    pub fn new(id: impl Into<ElementId>, text: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            selection: None,
            selecting: false,
            text_color: gpui::Hsla::default(),
            selection_color: gpui::Hsla::default(),
            mono: false,
            font_size: 12.0,
            row_index: None,
        }
    }

    pub fn text_color(mut self, color: Hsla) -> Self {
        self.text_color = color;
        self
    }

    pub fn selection_color(mut self, color: Hsla) -> Self {
        self.selection_color = color;
        self
    }

    pub fn mono(mut self, val: bool) -> Self {
        self.mono = val;
        self
    }

    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    pub fn row_index(mut self, idx: usize) -> Self {
        self.row_index = Some(idx);
        self
    }

    /// Coordinate-based character index at a given x-offset.
    /// Uses the font's average char width for monospace.
    /// For pixel-perfect accuracy, shape the text via `TextSystem::shape_line()`
    /// and use `ShapedLine::index_for_x()`.
    fn char_at_x(&self, x: f32) -> usize {
        let text_str = self.text.as_ref();
        if text_str.is_empty() {
            return 0;
        }
        let avg_char_w = self.font_size * 0.55;
        let raw_idx = (x / avg_char_w).round() as usize;
        raw_idx.min(text_str.len())
    }

    /// Compute index from pixel x coordinate relative to text start.
    pub fn index_for_x(&self, x: f32) -> usize {
        self.char_at_x(x.max(0.0))
    }
}

/// Render a piece of text with selection support.
pub fn render_selectable(
    el: &mut SelectableText,
    _window: &mut Window,
    _cx: &mut Context<'_, impl gpui::Focusable>,
) -> impl IntoElement {
    let text_str = el.text.clone();
    let sel = el.selection;
    let txt_color = el.text_color;
    let sel_color = el.selection_color;
    let is_mono = el.mono;
    let font_sz = el.font_size;

    let mut base = div()
        .id(el.id.clone())
        .cursor_text()
        .text_size(px(font_sz));

    if is_mono {
        base = base.font_family(SharedString::from("JetBrains Mono"));
    }

    let children: Vec<gpui::AnyElement> = if let Some((start, end)) = sel {
        let (s, e) = if start <= end { (start, end) } else { (end, start) };
        let s = s.min(text_str.len());
        let e = e.min(text_str.len());

        let before = &text_str[..s];
        let selected = &text_str[s..e];
        let after = &text_str[e..];

        vec![
            div()
                .text_color(txt_color)
                .child(SharedString::from(before))
                .into_any_element(),
            div()
                .bg(sel_color)
                .text_color(txt_color)
                .child(SharedString::from(selected))
                .into_any_element(),
            div()
                .text_color(txt_color)
                .child(SharedString::from(after))
                .into_any_element(),
        ]
    } else {
        vec![
            div()
                .text_color(txt_color)
                .child(text_str)
                .into_any_element(),
        ]
    };

    base.children(children)
}

/// Build a selectable text element for use in div-based layouts.
/// Returns an `AnyElement` with the selection highlight already baked in.
/// Does NOT handle mouse events — those should be attached at the parent level
/// via the `selection` callback.
pub fn selectable_text_element(
    id: ElementId,
    text: SharedString,
    selection: Option<(usize, usize)>,
    text_color: Hsla,
    sel_color: Hsla,
    font_size: f32,
) -> gpui::AnyElement {
    let txt = text.clone();
    if let Some((start, end)) = selection {
        let (s, e) = if start <= end { (start, end) } else { (end, start) };
        let s = s.min(txt.len());
        let e = e.min(txt.len());

        let before = txt[..s].to_string();
        let selected = txt[s..e].to_string();
        let after = txt[e..].to_string();

        div()
            .id(id)
            .cursor_text()
            .text_size(px(font_size))
            .font_family(SharedString::from("JetBrains Mono"))
            .child(div().text_color(text_color).child(SharedString::from(before)))
            .child(div().bg(sel_color).text_color(text_color).child(SharedString::from(selected)))
            .child(div().text_color(text_color).child(SharedString::from(after)))
            .into_any_element()
    } else {
        div()
            .id(id)
            .cursor_text()
            .text_size(px(font_size))
            .font_family(SharedString::from("JetBrains Mono"))
            .child(div().text_color(text_color).child(text))
            .into_any_element()
    }
}

/// Render a JSON value with per-row selection support.
/// Splits the value into before/selection/after segments based on the
/// `SelectionRange` for this row.
pub fn render_selectable_json_value(
    row_id: ElementId,
    text: &str,
    sel_range: Option<&SelectionRange>,
    row_index: usize,
    text_color: Hsla,
    sel_color: Hsla,
    font_size: f32,
) -> gpui::AnyElement {
    let (before, selected, after) = if let Some(range) = sel_range {
        range.offsets_for_row(row_index, text.len()).map_or_else(
            || (text.to_string(), String::new(), String::new()),
            |(s, e)| {
                let before = text[..s].to_string();
                let sel = text[s..e].to_string();
                let after = text[e..].to_string();
                (before, sel, after)
            },
        )
    } else {
        (text.to_string(), String::new(), String::new())
    };

    let has_sel = !selected.is_empty();

    div()
        .id(row_id)
        .cursor_text()
        .text_size(px(font_size))
        .font_family(SharedString::from("JetBrains Mono"))
        .child(div().text_color(text_color).child(SharedString::from(before)))
        .when(has_sel, |el| {
            el.child(div().bg(sel_color).text_color(text_color).child(SharedString::from(selected)))
        })
        .child(div().text_color(text_color).child(SharedString::from(after)))
        .into_any_element()
}
