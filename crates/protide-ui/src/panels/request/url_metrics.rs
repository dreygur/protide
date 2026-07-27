//! Text metrics for the URL input: cell width, horizontal scroll, and the
//! click-to-character mapping that has to agree with what was painted.

use super::*;

/// Font size the URL text is painted at. `render_url_text` must pass this to
/// the renderer: painting, horizontal scrolling and click-to-index mapping all
/// assume one cell width, and they disagree silently if it is written twice.
pub(super) const URL_FONT_SIZE: f32 = 13.0;

/// Monospace advance for [`URL_FONT_SIZE`].
pub(super) const URL_CHAR_WIDTH: f32 = URL_FONT_SIZE * 0.6;

/// Horizontal padding inside the URL input. `url_input_left` already includes
/// one of these; the visible text width is short by two.
pub(super) const URL_INPUT_PADDING: f32 = 14.0;

impl<E: WebSocketExecutor> RequestPanel<E> {
    /// Scroll actually applied to the painted URL text.
    ///
    /// An unfocused URL always paints from the start, so the offset only counts
    /// while focused. Click mapping has to agree with paint, so both go through
    /// here.
    pub(super) fn effective_url_scroll(&self, is_focused: bool) -> f32 {
        if is_focused {
            self.url_scroll_offset
        } else {
            0.0
        }
    }

    /// Character index at `x` measured from the start of the text.
    pub(super) fn index_for_x(&self, x: f32) -> usize {
        if x <= 0.0 {
            0
        } else {
            ((x / URL_CHAR_WIDTH) as usize).min(self.url.chars().count())
        }
    }

    /// Character index under a click at window-space `x`.
    ///
    /// `url_render_scroll` is the scroll the last painted frame used, so a
    /// scrolled URL maps the character the user actually clicked rather than
    /// the one that would sit there unscrolled.
    pub(super) fn index_for_event_x(&self, x: f32) -> usize {
        let click_x = (x - self.url_input_left + self.url_render_scroll).max(0.0);
        self.index_for_x(click_x)
    }

    /// Keep the caret inside the visible window after an edit or a move.
    pub(super) fn update_url_scroll(&mut self) {
        let visible_width = (self.url_input_width - URL_INPUT_PADDING * 2.0).max(60.0);
        let cursor_px = self.url_selection.end as f32 * URL_CHAR_WIDTH;

        if cursor_px < self.url_scroll_offset {
            self.url_scroll_offset = cursor_px;
        } else if cursor_px > self.url_scroll_offset + visible_width - URL_CHAR_WIDTH {
            self.url_scroll_offset = cursor_px - visible_width + URL_CHAR_WIDTH;
        }
        if self.url_scroll_offset < 0.0 {
            self.url_scroll_offset = 0.0;
        }
    }
}
