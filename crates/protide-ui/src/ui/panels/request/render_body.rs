//! Body tab rendering for RequestPanel

use std::ops::Range;

use gpui::{
    div, prelude::*, px, Context, IntoElement,
    ParentElement, Styled,
};

use crate::theme;
use crate::ui::components::icons::{
    icon, ICON_MD,
    ICON_FILE, ICON_FOLDER,
};
use protide_core::execution::ws::WebSocketExecutor;
use super::super::request_types::BodyType;
use super::RequestPanel;

impl<E: WebSocketExecutor> RequestPanel<E> {
    pub(super) fn render_body_tab(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        if self.body_type == BodyType::Form {
            return self.render_form_body(cx);
        }
        if self.body_type == BodyType::Binary {
            return self.render_binary_body(cx);
        }

        let theme = theme::current(cx);

        div()
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .child(
                div()
                    .h(px(40.0))
                    .w_full()
                    .flex()
                    .items_center()
                    .border_b_1()
                    .border_color(theme.colors.border)
                    .bg(theme.colors.bg_primary)
                    .child(self.render_body_type_btn("body-type-json", "JSON", BodyType::Json, cx))
                    .child(self.render_body_type_btn("body-type-raw", "Raw", BodyType::Raw, cx))
                    .child(self.render_body_type_btn("body-type-xml", "XML", BodyType::Xml, cx))
                    .child(self.render_body_type_btn("body-type-form", "Form", BodyType::Form, cx))
                    .child(self.render_body_type_btn("body-type-binary", "Binary", BodyType::Binary, cx))
                    .child(div().flex_1())
                    .child(
                        div()
                            .px(px(12.0))
                            .text_size(px(10.0))
                            .text_color(theme.colors.text_muted)
                            .child("request body")
                    )
            )
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .overflow_hidden()
                    .child(self.body_editor.clone())
            )
            .into_any_element()
    }

    fn render_body_type_btn(&self, id: &'static str, label: &'static str, bt: BodyType, cx: &Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let is_active = self.body_type == bt;
        div()
            .id(id)
            .h_full()
            .px(px(16.0))
            .flex()
            .items_center()
            .cursor_pointer()
            .border_b_2()
            .when(is_active, |el| el.border_color(theme.colors.accent))
            .when(!is_active, |el| el.border_color(gpui::transparent_black()).hover(|s| s.bg(theme.colors.hover_overlay)))
            .on_click(cx.listener(move |this, _, _, cx| { this.set_body_type(bt, cx); }))
            .child(div().text_size(px(13.0))
                .font_weight(if is_active { gpui::FontWeight::MEDIUM } else { gpui::FontWeight::NORMAL })
                .text_color(if is_active { theme.colors.text_primary } else { theme.colors.text_secondary })
                .child(label))
    }

    pub(super) fn render_binary_body(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let file_name = self.binary_file_path.as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string());
        let file_size = self.binary_file_path.as_ref()
            .and_then(|p| std::fs::metadata(p).ok())
            .map(|m| {
                let bytes = m.len();
                if bytes < 1024 { format!("{} B", bytes) }
                else if bytes < 1024 * 1024 { format!("{:.1} KB", bytes as f64 / 1024.0) }
                else { format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0)) }
            });

        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(12.0))
                    .when_some(file_name, |el, name| {
                        el.child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.0))
                                .child(icon(ICON_FILE, ICON_MD, theme.colors.accent))
                                .child(
                                    div()
                                        .text_size(px(13.0))
                                        .text_color(theme.colors.text_primary)
                                        .child(name)
                                )
                                .when_some(file_size, |el, size| {
                                    el.child(
                                        div()
                                            .text_size(px(11.0))
                                            .text_color(theme.colors.text_muted)
                                            .child(size)
                                    )
                                })
                        )
                    })
                    .when(self.binary_file_path.is_none(), |el| {
                        el.child(
                            div()
                                .text_size(px(12.0))
                                .text_color(theme.colors.text_muted)
                                .child("No file selected")
                        )
                    })
                    .child(
                        div()
                            .id("browse-binary-btn")
                            .px(px(16.0))
                            .py(px(8.0))
                            .bg(theme.colors.bg_tertiary)
                            .border_1()
                            .border_color(theme.colors.border)
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .cursor_pointer()
                            .hover(|s| s.border_color(theme.colors.accent))
                            .on_click(cx.listener(|this, _, _, cx| this.browse_binary_file(cx)))
                            .child(icon(ICON_FOLDER, ICON_MD, theme.colors.text_secondary))
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(theme.colors.text_secondary)
                                    .child(if self.binary_file_path.is_some() { "Change File" } else { "Browse File" })
                            )
                    )
            )
            .into_any_element()
    }
}
