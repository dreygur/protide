use gpui::{div, prelude::*, px, ClipboardItem, MouseButton, ParentElement, SharedString, Styled};

use crate::components::icons::{icon, ICON_COPY, ICON_SM, ICON_TEAM};
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
            .child(self.render_pairing_badge(theme))
            .into_any_element()
    }

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
}
