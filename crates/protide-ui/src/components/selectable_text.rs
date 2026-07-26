//! Selectable text rendering utilities.
//!
//! # Notify guard
//! Parents that attach mouse events MUST gate `cx.notify()` with `selection_changed()`.
//! Without the guard, every MouseMove triggers a full view re-render regardless of
//! whether the selection moved to a different character, causing continuous CPU spikes.

use gpui::{
    ElementId, Hsla, InteractiveElement, IntoElement, ParentElement, SharedString, Styled, div, px,
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
        Self {
            start_row,
            start_offset,
            end_row,
            end_offset,
        }
    }

    /// Returns the (start, end) byte offsets for a given row, if the row intersects selection.
    ///
    /// Offsets are clamped to the nearest valid UTF-8 char boundary at or before the
    /// requested position. This is a defensive backstop: offsets should already be
    /// genuine byte offsets by the time they reach here, but callers that slice the
    /// text directly (e.g. `render_selectable_json_value`) would panic on a stray
    /// mid-character offset, so we guard against that here rather than there.
    pub fn offsets_for_row(&self, row: usize, text: &str) -> Option<(usize, usize)> {
        let text_len = text.len();
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
            let s = floor_char_boundary(text, so.min(eo).min(text_len));
            let e = floor_char_boundary(text, so.max(eo).min(text_len));
            return Some((s, e));
        }
        // Multi-row: so is the offset within sr, eo is the offset within er.
        // They are in different rows, so min/max across them is meaningless.
        if row == sr {
            Some((floor_char_boundary(text, so.min(text_len)), text_len))
        } else if row == er {
            Some((0, floor_char_boundary(text, eo.min(text_len))))
        } else {
            Some((0, text_len))
        }
    }
}

/// Clamps `idx` to the nearest valid UTF-8 char boundary at or before `idx`.
/// Backstop for byte offsets that may not align to a char boundary (e.g. derived
/// from a coordinate space that no longer matches the current text). Stable-Rust
/// equivalent of the unstable `str::floor_char_boundary`.
pub(crate) fn floor_char_boundary(text: &str, idx: usize) -> usize {
    let mut i = idx.min(text.len());
    while i > 0 && !text.is_char_boundary(i) {
        i -= 1;
    }
    i
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
pub fn selection_changed(old: Option<(usize, usize)>, new_start: usize, new_end: usize) -> bool {
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
        .text_color(text_color); // inherited by all direct SharedString children

    if let Some((start, end)) = selection {
        let (s, e) = if start <= end {
            (start, end)
        } else {
            (end, start)
        };
        // Snap to a char boundary, not just into range: these offsets can be
        // stale (captured against a previous response's header value), and a
        // mid-character `&text[..s]` would panic the whole window.
        let s = floor_char_boundary(&text, s);
        let e = floor_char_boundary(&text, e);
        if s < e {
            base.child(SharedString::from(&text[..s]))
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
        .text_color(text_color); // inherited by plain SharedString children

    // Only compute offsets when the selection actually touches this row.
    if let Some((s, e)) = sel_range.and_then(|r| r.offsets_for_row(row_index, text)) {
        if s < e {
            // Non-empty selection: split into before / highlight / after.
            base.child(SharedString::from(&text[..s]))
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

#[cfg(test)]
mod tests {
    use super::*;

    /// "日本語" - three 3-byte chars, boundaries at 0/3/6/9.
    const CJK: &str = "日本語";
    /// One combining mark, one emoji, one ASCII - boundaries at 0/1/3/7/8.
    const MIXED: &str = "e\u{0301}🎉x";

    fn range(sr: usize, so: usize, er: usize, eo: usize) -> SelectionRange {
        SelectionRange::new(sr, so, er, eo)
    }

    // ── floor_char_boundary ──────────────────────────────────────────────────

    #[test]
    fn floor_snaps_back_to_the_start_of_the_character_it_lands_in() {
        for (idx, want) in [
            (0, 0),
            (1, 0),
            (2, 0),
            (3, 3),
            (4, 3),
            (5, 3),
            (6, 6),
            (9, 9),
        ] {
            assert_eq!(floor_char_boundary(CJK, idx), want, "idx {idx}");
        }
    }

    #[test]
    fn floor_clamps_offsets_past_the_end_of_the_string() {
        assert_eq!(floor_char_boundary(CJK, 9), CJK.len());
        assert_eq!(floor_char_boundary(CJK, 10), CJK.len());
        assert_eq!(floor_char_boundary(CJK, usize::MAX), CJK.len());
        assert_eq!(floor_char_boundary("", 0), 0);
        assert_eq!(floor_char_boundary("", usize::MAX), 0);
    }

    #[test]
    fn floor_always_yields_a_sliceable_offset() {
        for text in [CJK, MIXED, "", "ascii", "a\u{0301}"] {
            for idx in 0..=text.len() + 4 {
                let i = floor_char_boundary(text, idx);
                assert!(text.is_char_boundary(i), "{text:?} idx {idx} -> {i}");
                let _ = &text[..i]; // must not panic
                let _ = &text[i..];
            }
        }
    }

    // ── SelectionRange::offsets_for_row ──────────────────────────────────────

    #[test]
    fn rows_outside_the_selection_are_not_highlighted() {
        let sel = range(2, 0, 4, 3);
        assert_eq!(sel.offsets_for_row(1, CJK), None);
        assert_eq!(sel.offsets_for_row(5, CJK), None);
        assert!(sel.offsets_for_row(2, CJK).is_some());
        assert!(sel.offsets_for_row(4, CJK).is_some());
    }

    #[test]
    fn a_single_row_selection_normalises_a_right_to_left_drag() {
        let text = "abcdef";
        let forward = range(0, 1, 0, 4).offsets_for_row(0, text);
        let backward = range(0, 4, 0, 1).offsets_for_row(0, text);
        assert_eq!(forward, Some((1, 4)));
        assert_eq!(backward, forward, "drag direction must not change the span");
    }

    #[test]
    fn a_multi_row_selection_covers_the_tail_middle_and_head_of_its_rows() {
        let text = "abcdef";
        let sel = range(1, 2, 3, 4);
        assert_eq!(sel.offsets_for_row(1, text), Some((2, text.len())));
        assert_eq!(sel.offsets_for_row(2, text), Some((0, text.len())));
        assert_eq!(sel.offsets_for_row(3, text), Some((0, 4)));
    }

    #[test]
    fn a_multi_row_selection_dragged_upwards_keeps_each_offset_on_its_own_row() {
        // Dragging up means start_row > end_row: start_offset belongs to the
        // *lower* row. Swapping rows without swapping offsets would apply the
        // wrong offset to each row.
        let text = "abcdef";
        let up = range(3, 4, 1, 2);
        assert_eq!(up.offsets_for_row(1, text), Some((2, text.len())));
        assert_eq!(up.offsets_for_row(2, text), Some((0, text.len())));
        assert_eq!(up.offsets_for_row(3, text), Some((0, 4)));
        assert_eq!(up.offsets_for_row(0, text), None);
        assert_eq!(up.offsets_for_row(4, text), None);
    }

    #[test]
    fn offsets_past_the_end_of_a_shorter_row_are_clamped() {
        // Rows in a JSON tree have wildly different lengths; a selection that
        // starts on a long row must not index past a short one.
        let short = "ab";
        assert_eq!(range(0, 0, 0, 99).offsets_for_row(0, short), Some((0, 2)));
        assert_eq!(range(0, 99, 0, 99).offsets_for_row(0, short), Some((2, 2)));
        assert_eq!(range(0, 50, 2, 50).offsets_for_row(1, short), Some((0, 2)));
        assert_eq!(range(0, 50, 2, 50).offsets_for_row(2, short), Some((0, 2)));
    }

    #[test]
    fn offsets_are_always_char_boundaries_of_the_row_they_index() {
        for text in [CJK, MIXED, "", "ascii"] {
            for so in 0..=text.len() + 2 {
                for eo in 0..=text.len() + 2 {
                    for (row, sel) in [
                        (0, range(0, so, 0, eo)),
                        (1, range(0, so, 2, eo)),
                        (0, range(0, so, 2, eo)),
                        (2, range(0, so, 2, eo)),
                        (1, range(2, so, 0, eo)),
                    ] {
                        let Some((s, e)) = sel.offsets_for_row(row, text) else {
                            continue;
                        };
                        assert!(s <= e, "inverted span {s}..{e} in {text:?}");
                        assert!(text.is_char_boundary(s), "{text:?} start {s}");
                        assert!(text.is_char_boundary(e), "{text:?} end {e}");
                        let _ = &text[s..e]; // must not panic
                    }
                }
            }
        }
    }

    // ── selection_changed ────────────────────────────────────────────────────

    #[test]
    fn an_empty_first_selection_is_not_a_change() {
        // Gate for cx.notify(): a mouse-down that has not dragged yet must not
        // trigger a re-render on every subsequent identical move event.
        assert!(!selection_changed(None, 5, 5));
        assert!(selection_changed(None, 5, 6));
    }

    #[test]
    fn only_a_moved_endpoint_counts_as_a_change() {
        assert!(!selection_changed(Some((2, 7)), 2, 7));
        assert!(selection_changed(Some((2, 7)), 2, 8));
        assert!(selection_changed(Some((2, 7)), 3, 7));
        // Collapsing an existing selection is still a change worth redrawing.
        assert!(selection_changed(Some((2, 7)), 4, 4));
    }

    // ── element builders: must never panic on a stray offset ─────────────────

    fn build_selectable(text: &str, sel: Option<(usize, usize)>) {
        let _ = selectable_text_element(
            gpui::ElementId::Integer(0),
            SharedString::from(text.to_string()),
            sel,
            gpui::black(),
            gpui::white(),
            12.0,
        );
    }

    // FIXED: `selectable_text_element` clamped its offsets with `.min(text.len())`
    // only - it never snapped them to a char boundary, unlike every sibling that
    // slices row text (`offsets_for_row`, `copy_json_selection`). A header
    // selection is *not* cleared when a new response arrives (see
    // `ResponsePanel::set_response`), so offsets captured against one response's
    // header value get sliced against the next response's value; if the new value
    // is multi-byte the `&text[..s]` slice lands mid-character and panics the UI.
    #[test]
    fn a_stale_offset_from_a_previous_response_does_not_panic_the_header_row() {
        // Offsets 3/6 are valid boundaries in "日本語" but land inside the 'é'
        // and past the end of "abcdeé".
        for sel in [(0, 6), (3, 6), (6, 6), (0, 3), (3, 99), (99, 0)] {
            build_selectable("abcdeé", Some(sel));
        }
    }

    #[test]
    fn every_offset_pair_renders_a_selectable_row_without_panicking() {
        for text in [CJK, MIXED, "", "ascii"] {
            for s in 0..=text.len() + 2 {
                for e in 0..=text.len() + 2 {
                    build_selectable(text, Some((s, e)));
                }
            }
            build_selectable(text, None);
        }
    }

    #[test]
    fn every_offset_pair_renders_a_json_row_without_panicking() {
        for text in [CJK, MIXED, "", "ascii"] {
            for s in 0..=text.len() + 2 {
                for e in 0..=text.len() + 2 {
                    for sel in [range(0, s, 0, e), range(0, s, 2, e), range(2, s, 0, e)] {
                        for row in 0..4 {
                            let _ = render_selectable_json_value(
                                gpui::ElementId::Integer(0),
                                text,
                                Some(&sel),
                                row,
                                gpui::black(),
                                gpui::white(),
                                12.0,
                            );
                        }
                    }
                }
            }
        }
    }
}
