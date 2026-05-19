use std::time::{Duration, Instant};
use gpui::{
    div, prelude::*, px, ClipboardItem, InteractiveElement, IntoElement, MouseButton,
    ParentElement, SharedString, Styled,
};

use crate::theme;
use crate::components::icons::{
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

/// Connection state for the Join Peer flow
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionStatus {
    Idle,
    Handshaking,
    Connected,
    /// Handshake timed out or PAKE verification failed
    Error(String),
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
    /// Connection state for the join flow
    pub connection_status: ConnectionStatus,
    /// Flips every tick while Handshaking — drives pulsing border
    pub handshake_tick: bool,
}

impl Default for PresenceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PresenceManager {
    pub fn new() -> Self {
        Self {
            peers: Vec::new(),
            enabled: true,
            show_pairing: false,
            pairing_code: SharedString::default(),
            generated_code: SharedString::default(),
            connection_status: ConnectionStatus::Idle,
            handshake_tick: false,
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

    /// Generate a new pairing code
    pub fn generate_code(&mut self) {
        #[cfg(feature = "pake-auth")]
        {
            self.generated_code = SharedString::from(protide_core::sync::pake::generate_pairing_code());
            self.pairing_code = self.generated_code.clone();
        }
        #[cfg(not(feature = "pake-auth"))]
        {
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

    /// Flip the pulse tick — call once per second while Handshaking
    pub fn tick_handshake(&mut self) {
        if self.connection_status == ConnectionStatus::Handshaking {
            self.handshake_tick = !self.handshake_tick;
        }
    }

    /// Transition to Connected, update peer display
    pub fn set_connected(&mut self, peer_id: String, peer_name: String) {
        self.connection_status = ConnectionStatus::Connected;
        self.upsert_peer(peer_id, peer_name, PeerSource::P2P);
        self.show_pairing = false;
    }

    /// Reset connection state back to Idle
    pub fn reset_connection(&mut self) {
        self.connection_status = ConnectionStatus::Idle;
        self.handshake_tick = false;
    }

    /// Nearby peers (mDNS / UDP-discovered, not yet PAKE-authenticated)
    pub fn nearby_peers(&self) -> Vec<&Peer> {
        self.peers
            .iter()
            .filter(|p| p.source == PeerSource::UDPBroadcast || p.source == PeerSource::P2P)
            .collect()
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
                    // Copy code button — hover/cursor/click only when a code exists
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
            // ── Join Peer (interactive — provided by caller) ──────────────────
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
            // ── Peers Found Nearby (mDNS) — shown when peers visible ──────────
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
