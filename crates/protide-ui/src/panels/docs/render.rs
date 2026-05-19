use gpui::{
    Context, FontWeight, IntoElement, MouseButton, ParentElement, Render, Styled, Window,
    div, prelude::*, px,
};
use std::{fs, path::PathBuf};
use crate::theme;
use crate::components::icons::{
    ICON_CHEVRON_RIGHT, ICON_FILE, ICON_FOLDER, ICON_MD, ICON_SM, icon,
};
use super::DocsPanel;

impl DocsPanel {
    fn render_list_row(
        &self,
        item: &crate::panels::explorer::CollectionItem,
        depth: usize,
        idx: usize,
        selected: &Option<PathBuf>,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let theme = theme::current(cx);
        let is_folder = item.is_folder;
        let indent = px((depth * 12 + 8) as f32);
        let is_selected = !is_folder && selected.as_ref() == Some(&item.path);
        let name = item.name.trim_end_matches(".http").to_string();
        let method = item.method.clone();
        let path = item.path.clone();
        let path_for_try = item.path.clone();

        div()
            .id(("docs-row", idx))
            .w_full()
            .min_h(px(24.0))
            .flex()
            .items_center()
            .pl(indent)
            .pr(px(6.0))
            .py(px(2.0))
            .when(!is_folder, |el| {
                el.cursor_pointer()
                    .when(is_selected, |el| el.bg(theme.colors.accent.opacity(0.15)))
                    .when(!is_selected, |el| el.hover(|s| s.bg(theme.colors.bg_tertiary)))
                    .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                        this.selected_path = Some(path.clone());
                        if let Some(exp) = &this.explorer {
                            exp.update(cx, |panel, cx| {
                                panel.load_request_file(path_for_try.clone(), cx);
                            });
                        }
                        cx.notify();
                    }))
            })
            .when(is_folder, |el| {
                el.child(icon(ICON_CHEVRON_RIGHT, ICON_SM, theme.colors.text_muted))
                    .child(div().w(px(3.0)))
                    .child(icon(ICON_FOLDER, ICON_SM, theme.colors.text_muted))
                    .child(div().w(px(4.0)))
            })
            .when(!is_folder, |el| {
                el.child(div().w(px(12.0)))
                    .child(icon(ICON_FILE, ICON_SM, theme.colors.accent))
                    .child(div().w(px(4.0)))
            })
            .when_some(method, |el, m| {
                let color = theme.method_color(&m);
                el.child(
                    div()
                        .min_w(px(28.0))
                        .h(px(13.0))
                        .px(px(3.0))
                        .flex()
                        .items_center()
                        .bg(color.opacity(0.12))
                        .border_1()
                        .border_color(color.opacity(0.6))
                        .child(
                            div()
                                .text_size(px(8.0))
                                .font_weight(FontWeight::BOLD)
                                .font_family("JetBrains Mono")
                                .text_color(color)
                                .child(m),
                        ),
                )
                .child(div().w(px(3.0)))
            })
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .text_size(px(11.0))
                    .text_color(if is_folder {
                        theme.colors.text_secondary
                    } else {
                        theme.colors.text_primary
                    })
                    .font_weight(if is_folder { FontWeight::MEDIUM } else { FontWeight::NORMAL })
                    .child(name),
            )
    }

    fn render_detail_inner(&self, path: &PathBuf, cx: &Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let Ok(text) = fs::read_to_string(path) else {
            return div().p(px(16.0)).text_size(px(12.0))
                .text_color(theme.colors.status_client_error)
                .child("Failed to read file").into_any();
        };
        let Ok(requests) = http_parser::parse(&text) else {
            return div().p(px(16.0)).text_size(px(12.0))
                .text_color(theme.colors.status_client_error)
                .child("Failed to parse").into_any();
        };
        let Some(req) = requests.first() else {
            return div().p(px(16.0)).text_size(px(12.0))
                .text_color(theme.colors.text_muted)
                .child("No requests in file").into_any();
        };
        let name = req.meta.name.clone().unwrap_or_else(|| {
            path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default()
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
                div().text_size(px(15.0)).font_weight(FontWeight::BOLD)
                    .text_color(theme.colors.text_primary).child(name),
            )
            .when_some(req.meta.description.clone(), |el, desc| {
                el.child(
                    div().text_size(px(12.0)).text_color(theme.colors.text_secondary).child(desc),
                )
            })
            .child(
                div().flex().items_center().gap(px(8.0)).p(px(10.0))
                    .bg(theme.colors.bg_tertiary).border_1().border_color(theme.colors.border)
                    .child(
                        div().px(px(8.0)).py(px(3.0))
                            .bg(method_color.opacity(0.15))
                            .border_1().border_color(method_color.opacity(0.5))
                            .text_size(px(10.0)).font_weight(FontWeight::BOLD)
                            .font_family("JetBrains Mono").text_color(method_color)
                            .child(method),
                    )
                    .child(
                        div().flex_1().min_w(px(0.0)).overflow_hidden()
                            .text_size(px(11.0)).font_family("JetBrains Mono")
                            .text_color(theme.colors.text_primary)
                            .child(req.url.clone()),
                    ),
            )
            .when(!headers.is_empty(), |el| {
                el.child(
                    div().flex().flex_col().gap(px(4.0))
                        .child(
                            div().text_size(px(11.0)).font_weight(FontWeight::SEMIBOLD)
                                .text_color(theme.colors.text_secondary).child("Headers"),
                        )
                        .children(headers.iter().map(|h| {
                            div().flex().items_center().gap(px(8.0))
                                .py(px(3.0)).border_b_1()
                                .border_color(theme.colors.border.opacity(0.4))
                                .child(
                                    div().w(px(110.0)).flex_shrink_0().text_size(px(11.0))
                                        .font_family("JetBrains Mono")
                                        .text_color(theme.colors.text_secondary)
                                        .child(h.key.clone()),
                                )
                                .child(
                                    div().flex_1().min_w(px(0.0)).overflow_hidden()
                                        .text_size(px(11.0))
                                        .text_color(theme.colors.text_primary)
                                        .child(h.value.clone()),
                                )
                        })),
                )
            })
            .when_some(req.body.clone().filter(|b| !b.trim().is_empty()), |el, body| {
                el.child(
                    div().flex().flex_col().gap(px(4.0))
                        .child(
                            div().text_size(px(11.0)).font_weight(FontWeight::SEMIBOLD)
                                .text_color(theme.colors.text_secondary).child("Body"),
                        )
                        .child(
                            div().p(px(8.0)).bg(theme.colors.bg_primary)
                                .border_1().border_color(theme.colors.border)
                                .text_size(px(11.0)).font_family("JetBrains Mono")
                                .text_color(theme.colors.text_primary).child(body),
                        ),
                )
            })
            .into_any()
    }
}

impl Render for DocsPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let flat_items = if let Some(explorer) = &self.explorer {
            let exp = explorer.read(cx);
            DocsPanel::flatten(exp.collection_items(), 0)
        } else {
            Vec::new()
        };
        let selected = self.selected_path.clone();

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.colors.bg_secondary)
            .child(
                div().h(px(40.0)).w_full().flex().items_center().px(px(14.0))
                    .border_b_1().border_color(theme.colors.border)
                    .child(
                        div().text_size(px(12.0)).font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.colors.text_primary).child("API Explorer"),
                    ),
            )
            .child(
                div().flex_1().w_full().flex()
                    .child(
                        div()
                            .id("docs-list")
                            .w(px(160.0))
                            .h_full()
                            .flex_shrink_0()
                            .border_r_1()
                            .border_color(theme.colors.border)
                            .overflow_scroll()
                            .child(
                                div().w_full().flex().flex_col().pt(px(4.0)).children(
                                    flat_items
                                        .iter()
                                        .enumerate()
                                        .map(|(idx, (item, depth))| self.render_list_row(item, *depth, idx, &selected, cx)),
                                ),
                            ),
                    )
                    .child(
                        div()
                            .id("docs-detail")
                            .flex_1()
                            .h_full()
                            .overflow_scroll()
                            .child(
                                div().w_full().flex().flex_col()
                                    .when_some(selected.clone(), |el, path| {
                                        el.child(self.render_detail_inner(&path, cx))
                                    })
                                    .when(selected.is_none(), |el| {
                                        el.size_full().flex().items_center().justify_center()
                                            .child(
                                                div().text_size(px(12.0))
                                                    .text_color(theme.colors.text_muted)
                                                    .child("Select an endpoint"),
                                            )
                                    }),
                            ),
                    ),
            )
    }
}
