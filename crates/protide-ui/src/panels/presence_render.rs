use gpui::{div, prelude::*, px, ClipboardItem, MouseButton, ParentElement, SharedString, Styled};

use crate::components::icons::{icon, ICON_COPY, ICON_NETWORK, ICON_SM, ICON_TEAM};
use crate::theme;

use super::presence::PresenceManager;

impl PresenceManager {
    /// Render the presence bar: peer avatars + pairing code badge.
    /// Used in the title bar area of main_window.
    pub fn render_presence_bar(&self, theme: &theme::Theme) -> gpui::AnyElement {
        let enabled = self.enabled;
        let count = self.peers.len();

        div()
            .flex()
            .items_center()
            .gap(px(4.0))
            .mr(px(6.0))
            // Network status indicator
            .child(
                div()
                    .size(px(6.0))
                    .rounded_full()
                    .bg(if enabled {
                        theme.colors.sync_active
                    } else {
                        theme.colors.text_muted
                    })
            )
            // Peer avatar circles
            .children(self.peers.iter().take(3).map(|peer| {
                let initials = peer.initials();
                div()
                    .size(px(20.0))
                    .rounded_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(theme.colors.team_accent.opacity(0.2))
                    .border_1()
                    .border_color(theme.colors.team_accent.opacity(0.4))
                    .child(
                        div()
                            .text_size(px(8.0))
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(theme.colors.team_accent)
                            .child(initials)
                    )
            }))
            // "+N more" badge if more than 3 peers
            .when(count > 3, |el| {
                el.child(
                    div()
                        .px(px(4.0))
                        .h(px(18.0))
                        .flex()
                        .items_center()
                        .bg(theme.colors.bg_elevated)
                        .border_1()
                        .border_color(theme.colors.border)
                        .child(
                            div()
                                .text_size(px(9.0))
                                .text_color(theme.colors.text_secondary)
                                .child(format!("+{}", count - 3))
                        )
                )
            })
            // Pairing code badge (clickable)
            .child(self.render_pairing_badge(theme))
            .into_any_element()
    }

    /// Render the pairing code badge (shown in the presence bar)
    pub(super) fn render_pairing_badge(&self, theme: &theme::Theme) -> gpui::AnyElement {
        let code = self.pairing_code.clone();
        let has_code = !code.is_empty();

        div()
            .px(px(6.0))
            .h(px(20.0))
            .flex()
            .items_center()
            .gap(px(4.0))
            .bg(if self.show_pairing {
                theme.colors.team_accent.opacity(0.15)
            } else {
                theme.colors.bg_elevated
            })
            .border_1()
            .border_color(if self.show_pairing {
                theme.colors.team_accent
            } else {
                theme.colors.team_accent.opacity(0.35)
            })
            .hover(|s| s.border_color(theme.colors.team_accent))
            .child(icon(ICON_TEAM, 10.0, theme.colors.team_accent))
            .child(
                div()
                    .text_size(px(9.0))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .font_family("JetBrains Mono")
                    .text_color(if has_code {
                        theme.colors.team_accent
                    } else {
                        theme.colors.text_muted
                    })
                    .child(if has_code { code } else { SharedString::from("PAIR") })
            )
            .into_any_element()
    }

    /// Render the pairing flyout panel.
    /// `join_section` is the interactive "Join Peer" block built by the caller
    /// (needs MainWindow context for button handlers and TextInput entity).
    pub fn render_pairing_flyout(
        &self,
        theme: &theme::Theme,
        join_section: gpui::AnyElement,
    ) -> gpui::AnyElement {
        let code = self.pairing_code.clone();
        let has_code = !code.is_empty();
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
            // ── Header ────────────────────────────────────────────────────────
            .child(
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
            )
            // ── Your Code ─────────────────────────────────────────────────────
            .child(
                div()
                    .px(px(12.0))
                    .pt(px(10.0))
                    .pb(px(10.0))
                    .flex()
                    .flex_col()
                    .gap(px(6.0))
                    // Section label
                    .child(
                        div()
                            .text_size(px(9.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.colors.text_muted)
                            .child("YOUR CODE")
                    )
                    // Code display
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
                                    .child(if has_code { code.clone() } else { SharedString::from("------") })
                            )
                    )
                    // Copy code button - hover/cursor/click only when a code exists
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
                                            cx.write_to_clipboard(ClipboardItem::new_string(
                                                code.to_string(),
                                            ));
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
            )
            // ── Divider ───────────────────────────────────────────────────────
            .child(
                div()
                    .h(px(1.0))
                    .w_full()
                    .bg(theme.colors.team_accent.opacity(0.2))
            )
            // ── Join Peer (interactive - provided by caller) ──────────────────
            .child(
                div()
                    .px(px(12.0))
                    .pt(px(10.0))
                    .pb(px(10.0))
                    .flex()
                    .flex_col()
                    .gap(px(6.0))
                    .child(
                        div()
                            .text_size(px(9.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.colors.text_muted)
                            .child("JOIN PEER")
                    )
                    .child(join_section)
            )
            // ── Peers Found Nearby (mDNS) - shown when peers visible ──────────
            .when(!nearby.is_empty(), |el| {
                el
                    .child(
                        div()
                            .h(px(1.0))
                            .w_full()
                            .bg(theme.colors.team_accent.opacity(0.2))
                    )
                    .child(
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
                                    .child(
                                        div()
                                            .size(px(5.0))
                                            .rounded_full()
                                            .bg(theme.colors.sync_active)
                                    )
                                    .child(
                                        div()
                                            .text_size(px(9.0))
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .text_color(theme.colors.text_muted)
                                            .child("PEERS FOUND NEARBY")
                                    )
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
                    )
            })
            // ── Connected Peers ───────────────────────────────────────────────
            .when(!self.peers.is_empty(), |el| {
                el
                    .child(
                        div()
                            .h(px(1.0))
                            .w_full()
                            .bg(theme.colors.team_accent.opacity(0.2))
                    )
                    .child(
                        div()
                            .px(px(12.0))
                            .pt(px(8.0))
                            .pb(px(8.0))
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .text_size(px(9.0))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.colors.text_muted)
                                    .child("CONNECTED PEERS")
                            )
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
                    )
            })
            .into_any_element()
    }
}
