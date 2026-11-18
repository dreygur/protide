//! Save, Code and Import buttons at the end of the URL bar

use gpui::{Context, IntoElement, ParentElement, Styled, Window, div, prelude::*, px};

use super::RequestPanel;
use crate::components::icons::{ICON_ARROW_DOWN, ICON_CODE, ICON_MD, ICON_SM, icon};
use crate::components::{toolbar_btn, tooltip_text};
use crate::theme;
use protide_core::execution::ws::WebSocketExecutor;

/// Fits "Saved!" and the changed on disk dot, so the label swap never moves the URL field.
const SAVE_BTN_W: f32 = 64.0;
/// Below this window width Code and Import drop their labels, leaving the room to the URL.
const COMPACT_WINDOW_W: f32 = 1100.0;

impl<E: WebSocketExecutor> RequestPanel<E> {
    pub(super) fn render_url_bar_actions(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = theme::current(cx);
        let compact = f32::from(window.viewport_size().width) < COMPACT_WINDOW_W;
        let is_saved = self.save_feedback;
        let has_conflict = self.external_change_pending;

        div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .child(
                toolbar_btn("save-button", cx)
                    .w(px(SAVE_BTN_W))
                    .when(is_saved, |el| {
                        el.text_color(theme.colors.status_success)
                            .border_color(theme.colors.status_success.opacity(0.4))
                    })
                    .when(has_conflict && !is_saved, |el| {
                        el.text_color(theme.colors.status_redirect)
                            .border_color(theme.colors.status_redirect.opacity(0.4))
                            .tooltip(tooltip_text("File changed on disk"))
                            .child(
                                div()
                                    .size(px(6.0))
                                    .rounded_full()
                                    .bg(theme.colors.status_redirect),
                            )
                    })
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.save_request(cx);
                    }))
                    .child(if is_saved { "Saved!" } else { "Save" }),
            )
            .child(
                toolbar_btn("code-button", cx)
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.generate_code(this.codegen_language, window, cx);
                    }))
                    .child(icon(ICON_CODE, ICON_MD, theme.colors.text_secondary))
                    .when(compact, |el| el.tooltip(tooltip_text("Code")))
                    .when(!compact, |el| el.child("Code")),
            )
            .child(
                toolbar_btn("import-button", cx)
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.open_import_modal(window, cx);
                    }))
                    .child(icon(ICON_ARROW_DOWN, ICON_SM, theme.colors.text_secondary))
                    .when(compact, |el| el.tooltip(tooltip_text("Import")))
                    .when(!compact, |el| el.child("Import")),
            )
    }
}
