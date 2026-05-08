use std::time::{Duration, Instant};
use gpui::{
    div, prelude::*, px, ClipboardItem, Context, InteractiveElement, IntoElement, MouseButton,
    ParentElement, SharedString, Styled,
};

use crate::theme;
use crate::ui::components::icons::{
    icon, ICON_COPY, ICON_NETWORK, ICON_SM, ICON_TEAM,
};

/// A connected peer on the network
#[derive(Debug, Clone)]
pub struct Peer {
    pub id: String,
    pub name: String,
    pub last_seen: Instant,
    pub is_active: bool,
    /// What protocol the peer was discovered through
    pub source: PeerSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PeerSource {
    UDPBroadcast,
    P2P,
    FileSync,
}

impl Peer {
    pub fn new(id: String, name: String, source: PeerSource) -> Self {
        Self {
            id,
            name,
            last_seen: Instant::now(),
            is_active: true,
            source,
        }
    }

    /// Display initials (first letter of each word)
    pub fn initials(&self) -> String {
        self.name
            .split(|c: char| !c.is_alphanumeric())
            .filter_map(|w| w.chars().next())
            .take(2)
            .collect::<String>()
            .to_uppercase()
    }
}

/// Manages the list of known peers and their status.
/// UI component that integrates with the sync engine.
#[derive(Debug, Clone)]
pub struct PresenceManager {
    peers: Vec<Peer>,
    /// Whether collaboration is currently enabled
    pub enabled: bool,
    /// Show the pairing flyout
    pub show_pairing: bool,
    /// Current pairing code (generated or entered)
    pub pairing_code: SharedString,
    /// Last generated pairing code
    pub generated_code: SharedString,
}

impl PresenceManager {
    pub fn new() -> Self {
        Self {
            peers: Vec::new(),
            enabled: true,
            show_pairing: false,
            pairing_code: SharedString::default(),
            generated_code: SharedString::default(),
        }
    }

    /// Add or update a peer
    pub fn upsert_peer(&mut self, id: String, name: String, source: PeerSource) {
        if let Some(existing) = self.peers.iter_mut().find(|p| p.id == id) {
            existing.last_seen = Instant::now();
            existing.is_active = true;
            existing.name = name;
        } else {
            self.peers.push(Peer::new(id, name, source));
        }
    }

    /// Remove a peer
    pub fn remove_peer(&mut self, id: &str) {
        self.peers.retain(|p| p.id != id);
    }

    /// Reap stale peers (not seen for >30s)
    pub fn reap_stale(&mut self) {
        let cutoff = Instant::now() - Duration::from_secs(30);
        self.peers.retain(|p| p.last_seen > cutoff);
    }

    /// Get active peers
    pub fn active_peers(&self) -> &[Peer] {
        &self.peers
    }

    /// Number of active peers
    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }

    /// Toggle pairing flyout
    pub fn toggle_pairing(&mut self, cx: &mut Context<Self>) {
        self.show_pairing = !self.show_pairing;
        cx.notify();
    }

    /// Generate a new pairing code
    pub fn generate_code(&mut self) {
        #[cfg(feature = "pake-auth")]
        {
            self.generated_code = SharedString::from(protide_core::sync::pake::generate_pairing_code());
            self.pairing_code = self.generated_code.clone();
        }
        #[cfg(not(feature = "pake-auth"))]
        {
            // Fallback code format
            use std::time::{SystemTime, UNIX_EPOCH};
            let suffix = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64
                % 10000;
            self.generated_code = SharedString::from(format!("protide-{:04}", suffix));
            self.pairing_code = self.generated_code.clone();
        }
    }

    // ── Rendering ─────────────────────────────────────────────────────────────

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
    fn render_pairing_badge(&self, theme: &theme::Theme) -> gpui::AnyElement {
        let code = self.pairing_code.clone();
        let has_code = !code.is_empty();

        // The badge interacts with the flyout — clicking it toggles show_pairing.
        // Since we can't call cx.notify() here (no Context<Self>), this is a
        // visual-only element. The actual toggle is handled by the parent that
        // has the PresenceManager context.
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
                theme.colors.team_accent.opacity(0.4)
            } else {
                theme.colors.border
            })
            .hover(|s| s.border_color(theme.colors.team_accent.opacity(0.5)))
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

    /// Render the pairing flyout panel (code display, copy, QR placeholder).
    pub fn render_pairing_flyout(&self, theme: &theme::Theme) -> gpui::AnyElement {
        let code = self.pairing_code.clone();
        let has_code = !code.is_empty();

        div()
            .w(px(220.0))
            .bg(theme.colors.bg_secondary)
            .border_1()
            .border_color(theme.colors.border)
            .shadow_lg()
            .overflow_hidden()
            .flex()
            .flex_col()
            // Header
            .child(
                div()
                    .h(px(32.0))
                    .px(px(12.0))
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .bg(theme.colors.bg_primary)
                    .border_b_1()
                    .border_color(theme.colors.border)
                    .child(icon(ICON_NETWORK, ICON_SM, theme.colors.team_accent))
                    .child(
                        div()
                            .text_size(px(11.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.colors.text_primary)
                            .child("Pairing Code")
                    )
                    .child(div().flex_1())
                    // Peer count
                    .child(
                        div()
                            .px(px(4.0))
                            .py(px(1.0))
                            .bg(theme.colors.team_accent.opacity(0.1))
                            .text_size(px(9.0))
                            .text_color(theme.colors.team_accent)
                            .child(format!("{} peers", self.peers.len()))
                    )
            )
            // Code display
            .child(
                div()
                    .px(px(16.0))
                    .py(px(16.0))
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(12.0))
                    // Large code display
                    .child(
                        div()
                            .w_full()
                            .h(px(48.0))
                            .bg(theme.colors.bg_primary)
                            .border_1()
                            .border_color(theme.colors.border)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                div()
                                    .text_size(px(20.0))
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
                    // Copy code button
                    .child(
                        div()
                            .id("pairing-copy-btn")
                            .h(px(28.0))
                            .px(px(12.0))
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .bg(theme.colors.team_accent)
                            .hover(|s| s.opacity(0.85))
                            .when(has_code, {
                                let code = code.clone();
                                move |el| {
                                    let code = code.clone();
                                    el.cursor_pointer()
                                        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                            cx.write_to_clipboard(ClipboardItem::new_string(code.to_string()));
                                        })
                                }
                            })
                            .child(icon(ICON_COPY, ICON_SM, theme.colors.bg_primary))
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.colors.bg_primary)
                                    .child("Copy Code")
                            )
                    )
                    // Peer list section
                    .when(!self.peers.is_empty(), |el| {
                        el.child(
                            div()
                                .w_full()
                                .flex()
                                .flex_col()
                                .gap(px(2.0))
                                .child(
                                    div()
                                        .text_size(px(9.0))
                                        .text_color(theme.colors.text_muted)
                                        .child("Connected Peers")
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
            )
            .into_any_element()
    }
}
