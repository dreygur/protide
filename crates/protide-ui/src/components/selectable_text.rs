//! Selectable text rendering utilities.
//!
//! # Notify guard
//! Parents that attach mouse events MUST gate `cx.notify()` with `selection_changed()`.
//! Without the guard, every MouseMove triggers a full view re-render regardless of
//! whether the selection moved to a different character, causing continuous CPU spikes.

use gpui::{
    div, px, ElementId, Hsla, InteractiveElement, IntoElement, ParentElement, SharedString, Styled,
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
            // Both offsets are in the same row - min/max to normalize direction.
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


/// Returns `true` when the selection actually changes - use this to gate `cx.notify()`.
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


/// Build a selectable text element for use in div-based layouts.
/// Returns an `AnyElement` with the selection highlight already baked in.
/// Does NOT handle mouse events - those should be attached at the parent level.
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
        if s < e {
            base
                .child(SharedString::from(&text[..s]))
                .child(div().bg(sel_color).child(SharedString::from(&text[s..e])))
                .child(SharedString::from(&text[e..]))
                .into_any_element()
        } else {
            base.child(text).into_any_element()
        }
    } else {
        // Common case: no selection - single text node, zero extra allocations.
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
        if s < e {
            // Non-empty selection: split into before / highlight / after.
            base
                .child(SharedString::from(&text[..s]))
                .child(div().bg(sel_color).child(SharedString::from(&text[s..e])))
                .child(SharedString::from(&text[e..]))
                .into_any_element()
        } else {
            base.child(SharedString::from(text)).into_any_element()
        }
    } else {
        // Common case: no selection on this row - one allocation, one layout box.
        base.child(SharedString::from(text)).into_any_element()
    }
}
