use gpui::{div, prelude::*, px, ClipboardItem, MouseButton, ParentElement, Styled};

use crate::components::icons::{icon, ICON_COPY, ICON_NETWORK, ICON_SM};
use crate::theme;

use super::presence::PresenceManager;

impl PresenceManager {
    pub fn render_pairing_flyout(
        &self,
        theme: &theme::Theme,
        join_section: gpui::AnyElement,
    ) -> gpui::AnyElement {
        let nearby = self.nearby_peers();

        div()
            .w(px(260.0))
            .bg(theme.colors.bg_secondary)
            .border_1()
            .border_color(theme.colors.team_accent)
            .shadow_lg()
            .overflow_hidden()
            .flex()
            .flex_col()
            .child(self.render_flyout_header(theme))
            .child(self.render_flyout_your_code(theme))
            .child(divider(theme))
            .child(
                div()
                    .px(px(12.0))
                    .pt(px(10.0))
                    .pb(px(10.0))
                    .flex()
                    .flex_col()
                    .gap(px(6.0))
                    .child(section_label("JOIN PEER", theme))
                    .child(join_section)
            )
            .when(!nearby.is_empty(), |el| {
                el.child(divider(theme))
                  .child(self.render_flyout_nearby(theme, &nearby))
            })
            .when(!self.peers.is_empty(), |el| {
                el.child(divider(theme))
                  .child(self.render_flyout_peers(theme))
            })
            .into_any_element()
    }

    fn render_flyout_header(&self, theme: &theme::Theme) -> gpui::AnyElement {
        div()
            .h(px(32.0))
            .px(px(12.0))
            .flex()
            .items_center()
            .gap(px(6.0))
            .bg(theme.colors.bg_primary)
            .border_b_1()
            .border_color(theme.colors.team_accent.opacity(0.3))
            .child(icon(ICON_NETWORK, ICON_SM, theme.colors.team_accent))
            .child(
                div()
                    .text_size(px(11.0))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(theme.colors.text_primary)
                    .child("Collaboration")
            )
            .child(div().flex_1())
            .child(
                div()
                    .px(px(5.0))
                    .py(px(1.0))
                    .bg(theme.colors.team_accent.opacity(0.12))
                    .border_1()
                    .border_color(theme.colors.team_accent.opacity(0.25))
                    .text_size(px(9.0))
                    .text_color(theme.colors.team_accent)
                    .child(format!("{} peers", self.peers.len()))
            )
            .into_any_element()
    }

    fn render_flyout_your_code(&self, theme: &theme::Theme) -> gpui::AnyElement {
        let code = self.pairing_code.clone();
        let has_code = !code.is_empty();

        div()
            .px(px(12.0))
            .pt(px(10.0))
            .pb(px(10.0))
            .flex()
            .flex_col()
            .gap(px(6.0))
            .child(section_label("YOUR CODE", theme))
            .child(
                div()
                    .w_full()
                    .h(px(40.0))
                    .bg(theme.colors.bg_primary)
                    .border_1()
                    .border_color(theme.colors.team_accent.opacity(0.3))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .text_size(px(16.0))
                            .font_weight(gpui::FontWeight::BOLD)
                            .font_family("JetBrains Mono")
                            .text_color(if has_code {
                                theme.colors.team_accent
                            } else {
                                theme.colors.text_muted
                            })
                            .child(if has_code { code.clone() } else { gpui::SharedString::from("------") })
                    )
            )
            .child(
                div()
                    .id("pairing-copy-btn")
                    .h(px(26.0))
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(px(6.0))
                    .bg(theme.colors.team_accent.opacity(0.12))
                    .border_1()
                    .border_color(theme.colors.team_accent.opacity(0.3))
                    .when(has_code, {
                        let code = code.clone();
                        let accent = theme.colors.team_accent;
                        move |el| {
                            let code = code.clone();
                            el.cursor_pointer()
                                .hover(move |s| s.bg(accent.opacity(0.2)))
                                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                    cx.write_to_clipboard(ClipboardItem::new_string(code.to_string()));
                                })
                        }
                    })
                    .child(icon(ICON_COPY, ICON_SM, theme.colors.team_accent))
                    .child(
                        div()
                            .text_size(px(10.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.colors.team_accent)
                            .child("Copy Code")
                    )
            )
            .into_any_element()
    }

    fn render_flyout_nearby(&self, theme: &theme::Theme, nearby: &[&super::presence::Peer]) -> gpui::AnyElement {
        div()
            .px(px(12.0))
            .pt(px(8.0))
            .pb(px(8.0))
            .flex()
            .flex_col()
            .gap(px(4.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .child(div().size(px(5.0)).rounded_full().bg(theme.colors.sync_active))
                    .child(section_label("PEERS FOUND NEARBY", theme))
            )
            .children(nearby.iter().take(5).map(|peer| {
                div()
                    .w_full()
                    .h(px(20.0))
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .child(
                        div()
                            .size(px(14.0))
                            .rounded_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .bg(theme.colors.team_accent.opacity(0.15))
                            .child(
                                div()
                                    .text_size(px(6.0))
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_color(theme.colors.team_accent)
                                    .child(peer.initials())
                            )
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_size(px(10.0))
                            .text_color(theme.colors.text_secondary)
                            .child(peer.name.clone())
                    )
                    .child(
                        div()
                            .text_size(px(8.0))
                            .text_color(theme.colors.sync_active)
                            .child("nearby")
                    )
            }))
            .into_any_element()
    }

    fn render_flyout_peers(&self, theme: &theme::Theme) -> gpui::AnyElement {
        div()
            .px(px(12.0))
            .pt(px(8.0))
            .pb(px(8.0))
            .flex()
            .flex_col()
            .gap(px(4.0))
            .child(section_label("CONNECTED PEERS", theme))
            .children(self.peers.iter().map(|peer| {
                div()
                    .w_full()
                    .h(px(22.0))
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .child(
                        div()
                            .size(px(16.0))
                            .rounded_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .bg(theme.colors.team_accent.opacity(0.15))
                            .child(
                                div()
                                    .text_size(px(7.0))
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_color(theme.colors.team_accent)
                                    .child(peer.initials())
                            )
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_size(px(11.0))
                            .text_color(theme.colors.text_primary)
                            .child(peer.name.clone())
                    )
                    .child(
                        div()
                            .size(px(5.0))
                            .rounded_full()
                            .bg(if peer.is_active {
                                theme.colors.status_success
                            } else {
                                theme.colors.text_muted
                            })
                    )
            }))
            .into_any_element()
    }
}

fn divider(theme: &theme::Theme) -> gpui::AnyElement {
    div()
        .h(px(1.0))
        .w_full()
        .bg(theme.colors.team_accent.opacity(0.2))
        .into_any_element()
}

fn section_label(text: &'static str, theme: &theme::Theme) -> gpui::AnyElement {
    div()
        .text_size(px(9.0))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(theme.colors.text_muted)
        .child(text)
        .into_any_element()
}
