//! Import modal rendering for RequestPanel


use gpui::{
    div, prelude::*, px, Context, IntoElement,
    ParentElement, Styled,
};

use crate::theme;
use crate::components::icons::{
    icon, ICON_SM, ICON_MD,
    ICON_CLOSE, ICON_FOLDER,
};
use protide_core::execution::ws::WebSocketExecutor;
use super::RequestPanel;

impl<E: WebSocketExecutor> RequestPanel<E> {
    pub fn render_import_modal(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let import_error = self.import_error.clone();

        div()
            .id("import-modal-overlay")
            .absolute()
            .inset_0()
            .bg(theme.colors.overlay)
            .flex()
            .items_center()
            .justify_center()
            .on_click(cx.listener(|this, _, _, cx| {
                this.close_import_modal(cx);
            }))
            .child(
                div()
                    .id("import-modal")
                    .w(px(600.0))
                    .bg(theme.colors.bg_primary)
                    .border_1()
                    .border_color(theme.colors.border)
                    .shadow_lg()
                    .flex()
                    .flex_col()
                    .on_mouse_down(gpui::MouseButton::Left, cx.listener(|_, _, _, cx| {
                        cx.stop_propagation();
                    }))
                    .on_click(cx.listener(|_, _, _, cx| {
                        cx.stop_propagation();
                    }))
                    // Header
                    .child(
                        div()
                            .h(px(48.0))
                            .px(px(16.0))
                            .flex()
                            .items_center()
                            .justify_between()
                            .border_b_1()
                            .border_color(theme.colors.border)
                            .child(
                                div()
                                    .text_size(px(14.0))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.colors.text_primary)
                                    .child("Import Request")
                            )
                            .child(
                                div()
                                    .id("close-import-modal")
                                    .size(px(28.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_size(px(14.0))
                                    .text_color(theme.colors.text_muted)
                                    .cursor_pointer()
                                    .hover(|s| s.bg(theme.colors.bg_tertiary))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.close_import_modal(cx);
                                    }))
                                    .child(icon(ICON_CLOSE, ICON_SM, theme.colors.text_muted))
                            )
                    )
                    // Content
                    .child(
                        div()
                            .p(px(16.0))
                            .flex()
                            .flex_col()
                            .gap(px(12.0))
                            .child(
                                div()
                                    .w_full()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        div()
                                            .text_size(px(12.0))
                                            .text_color(theme.colors.text_secondary)
                                            .child("Paste cURL command or Postman collection:")
                                    )
                                    .child(
                                        div()
                                            .id("browse-import")
                                            .px(px(10.0))
                                            .py(px(5.0))
                                            .text_size(px(11.0))
                                            .text_color(theme.colors.text_secondary)
                                            .cursor_pointer()
                                            .border_1()
                                            .border_color(theme.colors.border)
                                            .flex()
                                            .items_center()
                                            .gap(px(5.0))
                                            .hover(|s| s.bg(theme.colors.bg_tertiary))
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.browse_import_file(cx);
                                            }))
                                            .child(icon(ICON_FOLDER, ICON_MD, theme.colors.text_secondary))
                                            .child("Browse...")
                                    )
                            )
                            .child(
                                div()
                                    .id("import-editor-wrap")
                                    .w_full()
                                    .h(px(220.0))
                                    .border_1()
                                    .border_color(theme.colors.border)
                                    .overflow_hidden()
                                    .child(self.import_editor.clone())
                            )
                            .when(import_error.is_some(), |el| {
                                el.child(
                                    div()
                                        .px(px(10.0))
                                        .py(px(8.0))
                                        .bg(theme.colors.status_client_error.opacity(0.1))
                                        .border_1()
                                        .border_color(theme.colors.status_client_error.opacity(0.3))
                                        .text_size(px(12.0))
                                        .text_color(theme.colors.status_client_error)
                                        .child(import_error.unwrap_or_default())
                                )
                            })
                    )
                    // Footer
                    .child(
                        div()
                            .h(px(56.0))
                            .px(px(16.0))
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap(px(8.0))
                            .border_t_1()
                            .border_color(theme.colors.border)
                            .child(
                                div()
                                    .id("clear-import")
                                    .px(px(14.0))
                                    .py(px(8.0))
                                    .text_size(px(12.0))
                                    .text_color(theme.colors.text_secondary)
                                    .cursor_pointer()
                                    .border_1()
                                    .border_color(theme.colors.border)
                                    .hover(|s| s.bg(theme.colors.bg_tertiary))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.set_import_text(String::new(), cx);
                                    }))
                                    .child("Clear")
                            )
                            .child(
                                div()
                                    .id("execute-import")
                                    .px(px(14.0))
                                    .py(px(8.0))
                                    .text_size(px(12.0))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.colors.bg_primary)
                                    .bg(theme.colors.accent)
                                    .cursor_pointer()
                                    .hover(|s| s.bg(theme.colors.accent_hover))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.execute_import(cx);
                                    }))
                                    .child("Import")
                            )
                    )
            )
            .into_any_element()
    }
}
