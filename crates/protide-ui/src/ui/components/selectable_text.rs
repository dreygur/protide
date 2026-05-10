//! Selectable text rendering utilities.
//!
//! # TextLayout caching
//! GPUI's `LineLayoutCache` caches shaped lines by `(text, font, size)` across frames.
//! `index_for_x` calls `window.text_system().shape_line()`, which hits this cache on
//! every subsequent call with the same arguments — no manual caching is needed here.
//! Invalidation is automatic: when content or font_size changes, the cache misses and
//! re-shapes. Across a selection drag the text is constant, so hit-testing is O(1).
//!
//! # Notify guard
//! Parents that attach mouse events MUST gate `cx.notify()` with `selection_changed()`.
//! Without the guard, every MouseMove triggers a full view re-render regardless of
//! whether the selection moved to a different character, causing continuous CPU spikes.

use gpui::{
    div, font, px, Context, ElementId, Hsla, InteractiveElement,
    IntoElement, ParentElement, SharedString, Styled, TextRun, Window,
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
        if sr == er {
            row == sr && offset >= so.min(eo) && offset < so.max(eo)
        } else if row == sr {
            // Canonical start row: selection runs from so to end of line.
            offset >= so
        } else if row == er {
            // Canonical end row: selection runs from start of line to eo.
            offset < eo
        } else {
            row > sr && row < er
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
            // Both offsets are in the same row — min/max to normalize direction.
            let s = so.min(eo).min(text_len);
            let e = so.max(eo).min(text_len);
            return Some((s, e));
        }
        // Multi-row: so is the offset within sr, eo is the offset within er.
        // They are in different rows, so min/max across them is meaningless.
        if row == sr {
            Some((so.min(text_len), text_len))
        } else if row == er {
            Some((0, eo.min(text_len)))
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

    fn index_for_x_inner(&self, x: f32, window: &Window) -> usize {
        if self.text.is_empty() {
            return 0;
        }
        let f = if self.mono { font("JetBrains Mono") } else { font(".SystemUIFont") };
        let run = TextRun {
            len: self.text.len(),
            font: f,
            color: Hsla::default(),
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let shaped = window
            .text_system()
            .shape_line(self.text.clone(), px(self.font_size), &[run], None);
        // ShapedLine derefs to Arc<LineLayout> derefs to LineLayout
        shaped.closest_index_for_x(px(x.max(0.0)))
    }

    /// Pixel-exact character boundary nearest to `x` (relative to text origin).
    ///
    /// `x` must be local to this element's top-left corner. Inside a scrollable
    /// container, pass `event.position.x - element_origin.x - scroll_offset_x`.
    ///
    /// Only call this while the mouse button is down (selection drag). Calling on
    /// every MouseMove without a down-state guard causes a shape_line hit + re-render
    /// on every frame even when no character boundary changes.
    ///
    /// Uses GPUI's shaped line — correct for ligatures, kerning, variable-width fonts.
    /// The internal `LineLayoutCache` makes repeated calls for the same text O(1).
    pub fn index_for_x(&self, x: f32, window: &Window) -> usize {
        #[cfg(debug_assertions)]
        let t0 = std::time::Instant::now();

        let result = self.index_for_x_inner(x, window);

        #[cfg(debug_assertions)]
        {
            let elapsed = t0.elapsed();
            if elapsed.as_millis() > 8 {
                eprintln!(
                    "[SelectableText] SLOW hit-test: {}ms (text_len={})",
                    elapsed.as_millis(),
                    self.text.len()
                );
            }
        }
        result
    }
}

/// Returns `true` when the selection actually changes — use this to gate `cx.notify()`.
///
/// Without this guard, every `MouseMove` triggers a full view re-render even when the
/// cursor hasn't advanced to a different character, causing continuous CPU spikes.
///
/// # Example
/// ```ignore
/// let new_idx = selectable.index_for_x(delta_x, window);
/// if selection_changed(self.sel, anchor, new_idx) {
///     self.sel = Some((anchor, new_idx));
///     cx.notify();
/// }
/// ```
pub fn selection_changed(
    old: Option<(usize, usize)>,
    new_start: usize,
    new_end: usize,
) -> bool {
    match old {
        None => new_start != new_end,
        Some((s, e)) => s != new_start || e != new_end,
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
/// Does NOT handle mouse events — those should be attached at the parent level.
///
/// Hot-path optimisation: the no-selection case (the vast majority of rows)
/// emits a single text node with no child divs, so GPUI lays out one box
/// instead of three. The selection case emits three children: two plain
/// SharedString nodes (inherit parent text_color, no extra div) plus one
/// highlight span that needs a wrapper div for the background color.
pub fn selectable_text_element(
    id: ElementId,
    text: SharedString,
    selection: Option<(usize, usize)>,
    text_color: Hsla,
    sel_color: Hsla,
    font_size: f32,
) -> gpui::AnyElement {
    let base = div()
        .id(id)
        .cursor_text()
        .text_size(px(font_size))
        .font_family(SharedString::from("JetBrains Mono"))
        .text_color(text_color);  // inherited by all direct SharedString children

    if let Some((start, end)) = selection {
        let (s, e) = if start <= end { (start, end) } else { (end, start) };
        let s = s.min(text.len());
        let e = e.min(text.len());
        base
            .child(SharedString::from(&text[..s]))
            .child(div().bg(sel_color).child(SharedString::from(&text[s..e])))
            .child(SharedString::from(&text[e..]))
            .into_any_element()
    } else {
        // Common case: no selection — single text node, zero extra allocations.
        base.child(text).into_any_element()
    }
}

/// Render a JSON value with per-row selection support.
///
/// Hot-path design: most rows have no active selection, so we take a fast path
/// that emits a single text node (no string splitting, one child div).  Only
/// rows whose range intersects the current SelectionRange pay the cost of three
/// children and two extra string allocations.
pub fn render_selectable_json_value(
    row_id: ElementId,
    text: &str,
    sel_range: Option<&SelectionRange>,
    row_index: usize,
    text_color: Hsla,
    sel_color: Hsla,
    font_size: f32,
) -> gpui::AnyElement {
    let base = div()
        .id(row_id)
        .cursor_text()
        .text_size(px(font_size))
        .font_family(SharedString::from("JetBrains Mono"))
        .text_color(text_color);  // inherited by plain SharedString children

    // Only compute offsets when the selection actually touches this row.
    if let Some((s, e)) = sel_range.and_then(|r| r.offsets_for_row(row_index, text.len())) {
        // Selection intersects this row: split into before / highlight / after.
        // before and after are plain SharedString nodes — they inherit text_color
        // from the parent div, so no per-segment wrapper div is needed.
        base
            .child(SharedString::from(&text[..s]))
            .child(div().bg(sel_color).child(SharedString::from(&text[s..e])))
            .child(SharedString::from(&text[e..]))
            .into_any_element()
    } else {
        // Common case: no selection on this row — one allocation, one layout box.
        base.child(SharedString::from(text)).into_any_element()
    }
}
