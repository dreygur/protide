use super::DocsPanel;
use crate::theme;
use gpui::{Context, FontWeight, ParentElement, Styled, div, prelude::*, px};
use std::{fs, path::PathBuf};

impl DocsPanel {
    pub(super) fn render_detail_inner(
        &self,
        path: &PathBuf,
        cx: &Context<Self>,
    ) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let Ok(text) = fs::read_to_string(path) else {
            return div()
                .p(px(16.0))
                .text_size(px(12.0))
                .text_color(theme.colors.status_client_error)
                .child("Failed to read file")
                .into_any();
        };
        let Ok(requests) = http_parser::parse(&text) else {
            return div()
                .p(px(16.0))
                .text_size(px(12.0))
                .text_color(theme.colors.status_client_error)
                .child("Failed to parse")
                .into_any();
        };
        let Some(req) = requests.first() else {
            return div()
                .p(px(16.0))
                .text_size(px(12.0))
                .text_color(theme.colors.text_muted)
                .child("No requests in file")
                .into_any();
        };
        let name = req.meta.name.clone().unwrap_or_else(|| {
            path.file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default()
        });
        let method = req.method.as_str().to_string();
        let method_color = theme.method_color(&method);
        let headers: Vec<_> = req.headers.iter().filter(|h| h.enabled).collect();

        div()
            .w_full()
            .flex()
            .flex_col()
            .p(px(16.0))
            .gap(px(12.0))
            .child(
                div()
                    .text_size(px(15.0))
                    .font_weight(FontWeight::BOLD)
                    .text_color(theme.colors.text_primary)
                    .child(name),
            )
            .when_some(req.meta.description.clone(), |el, desc| {
                el.child(
                    div()
                        .text_size(px(12.0))
                        .text_color(theme.colors.text_secondary)
                        .child(desc),
                )
            })
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .p(px(10.0))
                    .bg(theme.colors.bg_tertiary)
                    .border_1()
                    .border_color(theme.colors.border)
                    .child(
                        div()
                            .px(px(8.0))
                            .py(px(3.0))
                            .bg(method_color.opacity(0.15))
                            .border_1()
                            .border_color(method_color.opacity(0.5))
                            .text_size(px(10.0))
                            .font_weight(FontWeight::BOLD)
                            .font_family("JetBrains Mono")
                            .text_color(method_color)
                            .child(method),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .overflow_hidden()
                            .text_size(px(11.0))
                            .font_family("JetBrains Mono")
                            .text_color(theme.colors.text_primary)
                            .child(req.url.clone()),
                    ),
            )
            .when(!headers.is_empty(), |el| {
                el.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(4.0))
                        .child(
                            div()
                                .text_size(px(11.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(theme.colors.text_secondary)
                                .child("Headers"),
                        )
                        .children(headers.iter().map(|h| {
                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.0))
                                .py(px(3.0))
                                .border_b_1()
                                .border_color(theme.colors.border.opacity(0.4))
                                .child(
                                    div()
                                        .w(px(110.0))
                                        .flex_shrink_0()
                                        .text_size(px(11.0))
                                        .font_family("JetBrains Mono")
                                        .text_color(theme.colors.text_secondary)
                                        .child(h.key.clone()),
                                )
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w(px(0.0))
                                        .overflow_hidden()
                                        .text_size(px(11.0))
                                        .text_color(theme.colors.text_primary)
                                        .child(h.value.clone()),
                                )
                        })),
                )
            })
            .when_some(
                req.body.clone().filter(|b| !b.trim().is_empty()),
                |el, body| {
                    el.child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(theme.colors.text_secondary)
                                    .child("Body"),
                            )
                            .child(
                                div()
                                    .p(px(8.0))
                                    .bg(theme.colors.bg_primary)
                                    .border_1()
                                    .border_color(theme.colors.border)
                                    .text_size(px(11.0))
                                    .font_family("JetBrains Mono")
                                    .text_color(theme.colors.text_primary)
                                    .child(body),
                            ),
                    )
                },
            )
            .into_any()
    }
}
