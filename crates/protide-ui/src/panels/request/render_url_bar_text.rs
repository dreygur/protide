use gpui::Context;

use super::RequestPanel;
use super::url_metrics::URL_FONT_SIZE;
use crate::components::render_text_view_with_max_scrolled;
use crate::theme;
use protide_core::execution::ws::WebSocketExecutor;

impl<E: WebSocketExecutor> RequestPanel<E> {
    pub(super) fn render_url_text(&self, is_focused: bool, cx: &Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let scroll = self.effective_url_scroll(is_focused);
        render_text_view_with_max_scrolled(
            &self.url,
            &self.url_selection,
            is_focused,
            URL_FONT_SIZE,
            theme.colors.text_primary,
            Some("Enter request URL..."),
            theme.colors.text_muted,
            None,
            theme.colors.accent.opacity(0.25),
            scroll,
        )
    }
}
