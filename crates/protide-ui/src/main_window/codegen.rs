use gpui::{Context, FontWeight, IntoElement, ParentElement, SharedString, Styled, div, px, prelude::*};
use super::*;

impl MainWindow {
    pub(super) fn render_codegen_panel(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let panel = self.request_panel.read(cx);
        let editor = gpui_component::input::Input::new(&panel.codegen_editor).disabled(true).appearance(false);
        let current_lang = panel.codegen_language;
        let width = self.codegen_panel_width;

        use protide_core::codegen::Language as CodegenLanguage;
        let languages: &[(CodegenLanguage, &str)] = &[
            (CodegenLanguage::Curl, "cURL"),
            (CodegenLanguage::Python, "Python"),
            (CodegenLanguage::JavaScript, "JS"),
            (CodegenLanguage::Go, "Go"),
            (CodegenLanguage::Rust, "Rust"),
        ];

        div()
            .id("codegen-panel")
            .w(px(width))
            .h_full()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .bg(theme.colors.bg_secondary)
            .border_l_1()
            .border_color(theme.colors.border)
            .child(
                div()
                    .h(theme.sizes.toolbar)
                    .px(px(12.0))
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .flex_shrink_0()
                    .border_b_1()
                    .border_color(theme.colors.border)
                    .child(div().flex().items_center().gap(px(2.0)).flex_1().children(
                        languages.iter().map(|&(lang, label)| {
                            let is_active = lang == current_lang;
                            div()
                                .id(SharedString::from(format!("codegen-tab-{}", label)))
                                .px(px(8.0))
                                .py(px(3.0))
                                .text_size(px(11.0))
                                .font_weight(FontWeight::MEDIUM)
                                .cursor_pointer()
                                .when(is_active, |el| {
                                    el.bg(theme.colors.accent.opacity(0.15))
                                        .text_color(theme.colors.accent)
                                        .border_1()
                                        .border_color(theme.colors.accent.opacity(0.3))
                                })
                                .when(!is_active, |el| {
                                    el.text_color(theme.colors.text_secondary).hover(|s| {
                                        s.bg(theme.colors.bg_tertiary)
                                            .text_color(theme.colors.text_primary)
                                    })
                                })
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.request_panel
                                        .update(cx, |panel, cx| panel.generate_code(lang, window, cx));
                                }))
                                .child(label)
                        }),
                    ))
                    .child(
                        div()
                            .id("codegen-copy")
                            .h(px(28.0))
                            .px(px(10.0))
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            .text_size(px(11.0))
                            .text_color(theme.colors.text_secondary)
                            .cursor_pointer()
                            .bg(theme.colors.bg_elevated)
                            .border_1()
                            .border_color(theme.colors.border)
                            .hover(|s| s.bg(theme.colors.bg_tertiary))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.request_panel
                                    .update(cx, |panel, cx| panel.copy_generated_code(cx));
                            }))
                            .child(icon(ICON_COPY, ICON_SM, theme.colors.text_secondary))
                            .child("Copy"),
                    )
                    .child(
                        div()
                            .id("codegen-close")
                            .size(px(28.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .text_color(theme.colors.text_muted)
                            .hover(|s| {
                                s.bg(theme.colors.bg_elevated)
                                    .text_color(theme.colors.text_primary)
                            })
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.request_panel
                                    .update(cx, |panel, cx| panel.close_codegen_panel(cx));
                            }))
                            .child(icon(ICON_CLOSE, ICON_SM, theme.colors.text_muted)),
                    ),
            )
            .child(div().flex_1().overflow_hidden().child(editor))
    }
}
