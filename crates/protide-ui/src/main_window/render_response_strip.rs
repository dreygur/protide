use gpui::{div, px, MouseButton, IntoElement, ParentElement, Styled, prelude::*};
use crate::theme;
use crate::components::icons::{icon, ICON_SM, ICON_CHEVRON_DOWN, ICON_CHEVRON_RIGHT};
use crate::panels::format_size;
use super::MainWindow;

impl MainWindow {
    /// Full-width accordion strip between request and response panels.
    /// Shows collapse chevron, "Response" label, and live status info on the right.
    pub(super) fn render_response_strip(
        &self,
        collapsed: bool,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let theme = theme::current(cx);

        // Capture status data before any rendering borrows
        let (resp_loading, resp_summary, resp_error) = {
            let panel = self.response_panel.read(cx);
            let loading = panel.is_loading();
            let summary = panel.last_response_summary()
                .map(|(s, st, ms, sz)| (s, st.to_string(), ms, sz));
            let error = panel.last_error().map(|e| {
                if e.len() > 42 { format!("{}…", &e[..40]) } else { e.to_string() }
            });
            (loading, summary, error)
        };

        let status_chip = {
            let tc = theme.clone();
            if resp_loading {
                div()
                    .text_size(px(10.0))
                    .text_color(tc.colors.text_muted)
                    .child("Sending…")
                    .into_any_element()
            } else if let Some(err) = resp_error {
                div()
                    .flex()
                    .items_center()
                    .px(px(6.0))
                    .py(px(2.0))
                    .bg(tc.colors.status_client_error.opacity(0.10))
                    .text_size(px(10.0))
                    .text_color(tc.colors.status_client_error)
                    .child(err)
                    .into_any_element()
            } else if let Some((status, st, ms, sz)) = resp_summary {
                let sc = tc.status_color(status);
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        div()
                            .px(px(6.0))
                            .py(px(2.0))
                            .bg(sc.opacity(0.12))
                            .text_size(px(10.0))
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(sc)
                            .child(format!("{} {}", status, st))
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(tc.colors.text_muted)
                            .child(format!("{}ms", ms))
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(tc.colors.text_muted)
                            .child(format_size(sz))
                    )
                    .into_any_element()
            } else {
                div().into_any_element()
            }
        };

        div()
            .id("response-accordion-header")
            .w_full()
            .h(theme.sizes.panel_header)
            .flex_shrink_0()
            .flex()
            .items_center()
            .px(px(12.0))
            .gap(px(crate::theme::sizes::CHEVRON_ICON_GAP))
            .bg(theme.colors.bg_secondary)
            // top border when collapsed (no drag strip above), bottom border always (separates from tabs)
            .when(collapsed, |el| el.border_t_1().border_color(theme.colors.border))
            .border_b_1()
            .border_color(theme.colors.border)
            .cursor_pointer()
            .hover(|s| s.bg(theme.colors.bg_tertiary))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.response_collapsed = !this.response_collapsed;
                    cx.notify();
                }),
            )
            .child(icon(
                if collapsed { ICON_CHEVRON_RIGHT } else { ICON_CHEVRON_DOWN },
                ICON_SM,
                theme.colors.text_muted,
            ))
            .child(
                div()
                    .ml(px(crate::theme::sizes::ICON_TEXT_GAP))
                    .text_size(px(11.0))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(theme.colors.text_secondary)
                    .child("Response")
            )
            .child(div().flex_1())
            .child(status_chip)
    }
}
