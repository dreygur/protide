//! Main application window

use std::path::PathBuf;

use gpui::{
    App, Context, Entity, FocusHandle, FontWeight, InteractiveElement, IntoElement, KeyBinding,
    MouseButton, ParentElement, Render, SharedString, Styled, WeakEntity, Window, div, prelude::*,
    px,
};

gpui::actions!(
    main_window,
    [
        SendRequest,
        SaveRequest,
        ToggleSidebar,
        ToggleMockServer,
        ToggleConsole,
        ShowHelp,
        ShowAbout,
        DismissOverlay,
        Quit
    ]
);

pub fn register_keybindings(cx: &mut gpui::App) {
    cx.bind_keys([
        KeyBinding::new("ctrl-enter", SendRequest, None),
        KeyBinding::new("ctrl-s", SaveRequest, None),
        KeyBinding::new("ctrl-b", ToggleSidebar, None),
        KeyBinding::new("ctrl-shift-m", ToggleMockServer, None),
        KeyBinding::new("ctrl-shift-c", ToggleConsole, None),
        KeyBinding::new("f1", ShowHelp, None),
        KeyBinding::new("ctrl-shift-a", ShowAbout, None),
        KeyBinding::new("escape", DismissOverlay, None),
        KeyBinding::new("ctrl-q", Quit, None),
    ]);
}

use super::panels::presence::{ConnectionStatus, PeerSource, PresenceManager};
use super::components::{TextInput, TextInputStyle};
use super::panels::{ConsoleEntry, ConsolePanel, ExplorerPanel, MockServerPanel, RequestPanel, ResponsePanel};
use crate::theme;
use protide_core::sync::{SyncEngine, SyncEvent};
use crate::ui::components::icons::{
    ICON_CLOSE, ICON_COPY, ICON_FOLDER, ICON_MAXIMIZE, ICON_MD, ICON_MENU, ICON_MINIMIZE,
    ICON_REFRESH, ICON_SETTINGS, ICON_SM, ICON_WINDOW_CLOSE, icon,
};
use crate::ui::components::modal::{ModalKind, ModalState, render_modal_shell};

/// Pending action for confirm modals
#[derive(Clone, Debug, Default)]
enum ModalPending {
    #[default]
    None,
    ExplorerDelete(PathBuf),
}

/// Main window containing the application layout
pub struct MainWindow {
    explorer: Entity<ExplorerPanel>,
    request_panel: Entity<RequestPanel>,
    response_panel: Entity<ResponsePanel>,
    mock_server_panel: Entity<MockServerPanel>,
    console_panel: Entity<ConsolePanel>,
    show_console: bool,
    console_height: f32,
    drag_console: Option<(f32, f32)>,
    show_mock_server: bool,
    sidebar_collapsed: bool,
    sidebar_width: f32,
    request_height: f32,
    mock_server_width: f32,
    codegen_panel_width: f32,
    drag_sidebar: Option<(f32, f32)>,
    drag_response: Option<(f32, f32)>,
    drag_mock_server: Option<(f32, f32)>,
    drag_codegen: Option<(f32, f32)>,
    modal: Option<ModalState>,
    modal_pending: ModalPending,
    show_help: bool,
    show_about: bool,
    focus: FocusHandle,
    /// Which title-bar menu is open (0=Protide, 1=Request, 2=View, 3=Help)
    open_menu: Option<u8>,
    /// Keeps the on_app_quit subscription alive for the lifetime of the window.
    _quit_sub: gpui::Subscription,
    /// Collaboration presence manager
    presence: PresenceManager,
    /// Sync engine for peer discovery and CRDT sync
    sync_engine: Option<SyncEngine>,
    /// Text input for the "Join Peer" pairing code field
    join_input: Entity<TextInput>,
    /// When the current PAKE handshake was initiated (for 10-second timeout)
    handshake_started: Option<std::time::Instant>,
    /// Last time cx.notify() was called from the P2P sync poller — used to throttle redraws
    last_p2p_notify: std::time::Instant,
}

impl MainWindow {
    pub fn build(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let main_window_weak: WeakEntity<MainWindow> = cx.entity().downgrade();
        let explorer = cx.new(|cx| ExplorerPanel::new(cx, main_window_weak.clone()));
        let response_panel = cx.new(|cx| ResponsePanel::new(cx));
        let response_panel_clone = response_panel.clone();
        let request_panel = cx.new(|cx| RequestPanel::new(cx, response_panel_clone));
        let mock_server_panel = cx.new(|cx| MockServerPanel::new(cx, main_window_weak));
        let console_panel = cx.new(|cx| ConsolePanel::new(cx));

        // Connect explorer to request panel for history loading
        let request_panel_clone = request_panel.clone();
        explorer.update(cx, |explorer, cx| {
            explorer.set_request_panel(request_panel_clone, cx);
        });

        // Connect request panel to explorer for environment variable substitution
        let explorer_clone = explorer.clone();
        request_panel.update(cx, |panel, cx| {
            panel.set_explorer_panel(explorer_clone, cx);
        });

        // Connect console panel to request panel for request logging
        let console_clone = console_panel.clone();
        request_panel.update(cx, |panel, cx| {
            panel.set_console_panel(console_clone, cx);
        });

        // ── Session restore ──────────────────────────────────────────────────
        let session = crate::session::load();
        if let Some(workspace) = session.current_workspace.clone().filter(|p| p.is_dir()) {
            let ws_key = workspace.to_string_lossy().to_string();
            let saved_entry = session.workspaces.get(&ws_key).cloned();
            explorer.update(cx, |exp, cx| {
                exp.init_workspace(workspace, saved_entry, cx);
            });
        }

        // ── Save session on quit ─────────────────────────────────────────────
        // on_app_quit fires for every quit path (Ctrl+Q, window × button, OS signal).
        // The returned Subscription must stay alive; we store it on the stack
        // here and then move it into the Self struct below.
        let quit_sub = cx.on_app_quit(|this: &mut Self, cx| {
            let state = this.capture_session(cx);
            async move { crate::session::save_sync(&state); }
        });

        let join_input = cx.new(|cx| {
            TextInput::new(cx, "join-code-input")
                .placeholder("enter pairing code…")
                .style(TextInputStyle::compact())
        });

        let mut presence = PresenceManager::new();
        presence.generate_code();

        // Initialize sync engine
        let node_name = std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_else(|_| "developer".into());
        let pairing_code_str = if presence.pairing_code.is_empty() {
            None
        } else {
            Some(presence.pairing_code.to_string())
        };
        let mut sync_engine = SyncEngine::new(protide_core::sync::SyncConfig {
            node_name,
            p2p_enabled: true,
            live_probe_enabled: true,
            pairing_code: pairing_code_str,
            ..Default::default()
        });
        let _ = sync_engine.init();
        let sync_engine = Some(sync_engine);

        // Periodic sync event polling (every 1 second)
        let poll_weak = cx.entity().downgrade();
        cx.spawn(async move |_, cx| {
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(1000))
                    .await;
                let weak = poll_weak.clone();
                weak.update(cx, |this, cx| {
                    this.poll_sync_events(cx);
                }).ok();
            }
        }).detach();

        Self {
            explorer,
            request_panel,
            response_panel,
            mock_server_panel,
            console_panel,
            show_console: false,
            console_height: 160.0,
            drag_console: None,
            show_mock_server: false,
            sidebar_collapsed: false,
            sidebar_width: crate::prefs::get_f32("main.sidebar_width", 250.0),
            request_height: crate::prefs::get_f32("main.request_height", 320.0),
            mock_server_width: crate::prefs::get_f32("main.mock_server_width", 320.0),
            codegen_panel_width: crate::prefs::get_f32("main.codegen_panel_width", 400.0),
            drag_sidebar: None,
            drag_response: None,
            drag_mock_server: None,
            drag_codegen: None,
            modal: None,
            modal_pending: ModalPending::None,
            show_help: false,
            show_about: false,
            focus: cx.focus_handle(),
            open_menu: None,
            _quit_sub: quit_sub,
            presence,
            sync_engine,
            join_input,
            handshake_started: None,
            last_p2p_notify: std::time::Instant::now(),
        }
    }

    fn toggle_console(&mut self, cx: &mut Context<Self>) {
        self.show_console = !self.show_console;
        cx.notify();
    }

    /// Build a full SessionState snapshot from the current live editor state.
    fn capture_session(&self, cx: &Context<Self>) -> crate::session::SessionState {
        let mut session = crate::session::load();
        let explorer = self.explorer.read(cx);

        if let Some(workspace) = explorer.workspace_path().cloned() {
            let draft = self.request_panel.read(cx).capture_draft(cx);
            let key   = workspace.to_string_lossy().to_string();
            let entry = session.workspaces.entry(key).or_default();
            entry.active_file      = explorer.selected_item().cloned();
            entry.draft            = Some(draft);
            entry.expanded_folders = explorer.collect_expanded();
            entry.active_env       = explorer.env_state().active().map(|e| e.name.clone());
            session.current_workspace = Some(workspace);
        }

        session
    }

    pub fn toggle_sidebar(&mut self, cx: &mut Context<Self>) {
        self.sidebar_collapsed = !self.sidebar_collapsed;
        cx.notify();
    }

    fn toggle_mock_server(&mut self, cx: &mut Context<Self>) {
        self.show_mock_server = !self.show_mock_server;
        cx.notify();
    }

    /// Initiate a PAKE handshake using the current join_input text as the shared code.
    fn connect_peer(&mut self, cx: &mut Context<Self>) {
        let code = self.join_input.read(cx).get_text().trim().to_string();
        if code.is_empty() {
            return;
        }
        self.presence.connection_status = ConnectionStatus::Handshaking;
        self.handshake_started = Some(std::time::Instant::now());
        cx.notify();

        if let Some(ref mut engine) = self.sync_engine {
            match engine.initiate_handshake(&code) {
                Ok(()) => {
                    self.console_panel.update(cx, |panel, cx| {
                        panel.log(ConsoleEntry::team(format!("Handshaking with code: {}", code)), cx);
                    });
                }
                Err(e) => {
                    self.console_panel.update(cx, |panel, cx| {
                        panel.log(ConsoleEntry::team(format!("Handshake error: {}", e)), cx);
                    });
                    self.presence.reset_connection();
                    cx.notify();
                }
            }
        }
    }

    /// Read the system clipboard and, if it contains text, paste it into the join
    /// input field and immediately attempt to connect.
    fn paste_and_join(&mut self, cx: &mut Context<Self>) {
        if let Some(item) = cx.read_from_clipboard() {
            let text = item.text().unwrap_or_default().to_string();
            self.join_input.update(cx, |input, input_cx| {
                input.set_text(text, input_cx);
            });
            self.connect_peer(cx);
        }
    }

    /// Build the flyout panel that drops down from the pairing badge.
    fn render_pairing_flyout_panel(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let status = self.presence.connection_status.clone();
        let tick = self.presence.handshake_tick;

        let neon = gpui::hsla(120.0 / 360.0, 1.0, 0.45, 1.0);

        // Single match → all connect-button properties + interactivity flag.
        // Hover and click are only wired when interactive = true, ensuring
        // .hover() is called at most once per element and never during Handshaking.
        let (connect_bg, connect_border, connect_text_color, connect_label, interactive) =
            match (&status, tick) {
                (ConnectionStatus::Handshaking, true) => {
                    (neon.opacity(0.15), neon, neon, "Connecting…", false)
                }
                (ConnectionStatus::Handshaking, false) => {
                    (neon.opacity(0.15), neon.opacity(0.25), neon, "Connecting…", false)
                }
                _ => (
                    theme.colors.team_accent.opacity(0.12),
                    theme.colors.team_accent,
                    theme.colors.team_accent,
                    "Connect",
                    true,
                ),
            };

        let error_msg: Option<String> = match &status {
            ConnectionStatus::Error(msg) => Some(msg.clone()),
            _ => None,
        };

        // Connect button: hover/cursor/click added only in the interactive branch
        let connect_btn_base = div()
            .id("join-connect-btn")
            .flex_1()
            .h(px(26.0))
            .flex()
            .items_center()
            .justify_center()
            .bg(connect_bg)
            .border_1()
            .border_color(connect_border)
            .child(
                div()
                    .text_size(px(10.0))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(connect_text_color)
                    .child(connect_label),
            );

        let connect_btn: gpui::AnyElement = if interactive {
            connect_btn_base
                .cursor_pointer()
                .hover(|s| s.opacity(0.85))
                .on_click(cx.listener(|this, _, _, cx| this.connect_peer(cx)))
                .into_any_element()
        } else {
            connect_btn_base.into_any_element()
        };

        // Paste button: disabled (dimmed, no hover/click) while handshaking
        let paste_btn_base = div()
            .id("join-paste-btn")
            .flex_1()
            .h(px(26.0))
            .flex()
            .items_center()
            .justify_center()
            .bg(theme.colors.bg_tertiary)
            .border_1()
            .border_color(theme.colors.border)
            .child(
                div()
                    .text_size(px(10.0))
                    .text_color(if interactive {
                        theme.colors.text_secondary
                    } else {
                        theme.colors.text_muted
                    })
                    .child("Paste & Join"),
            );

        let paste_btn: gpui::AnyElement = if interactive {
            paste_btn_base
                .cursor_pointer()
                .hover(|s| s.bg(theme.colors.bg_elevated))
                .on_click(cx.listener(|this, _, _, cx| this.paste_and_join(cx)))
                .into_any_element()
        } else {
            paste_btn_base.into_any_element()
        };

        let join_section = div()
            .flex()
            .flex_col()
            .gap(px(6.0))
            .child(self.join_input.clone())
            .child(div().flex().gap(px(6.0)).child(paste_btn).child(connect_btn))
            .when_some(error_msg, |el, msg| {
                let error_color = theme.colors.error;
                el.child(
                    div()
                        .w_full()
                        .px(px(8.0))
                        .py(px(5.0))
                        .bg(error_color.opacity(0.08))
                        .border_1()
                        .border_color(error_color.opacity(0.35))
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .child(
                            div()
                                .flex_1()
                                .text_size(px(10.0))
                                .text_color(error_color)
                                .child(msg),
                        )
                        .child(
                            div()
                                .id("handshake-retry-btn")
                                .px(px(6.0))
                                .py(px(2.0))
                                .text_size(px(9.0))
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(error_color)
                                .cursor_pointer()
                                .hover(move |s| s.bg(error_color.opacity(0.15)))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.presence.reset_connection();
                                    this.handshake_started = None;
                                    cx.notify();
                                }))
                                .child("Retry"),
                        ),
                )
            })
            .into_any_element();

        self.presence.render_pairing_flyout(&theme, join_section)
    }

    pub fn show_modal(&mut self, state: ModalState, cx: &mut Context<Self>) {
        self.modal = Some(state);
        self.modal_pending = ModalPending::None;
        cx.notify();
    }

    pub fn show_confirm_delete(
        &mut self,
        state: ModalState,
        path: PathBuf,
        cx: &mut Context<Self>,
    ) {
        self.modal = Some(state);
        self.modal_pending = ModalPending::ExplorerDelete(path);
        cx.notify();
    }

    fn dismiss_modal(&mut self, cx: &mut Context<Self>) {
        self.modal = None;
        self.modal_pending = ModalPending::None;
        cx.notify();
    }

    fn dismiss_overlay(&mut self, cx: &mut Context<Self>) {
        if self.modal.is_some() {
            self.modal = None;
            self.modal_pending = ModalPending::None;
        } else if self.show_help {
            self.show_help = false;
        } else if self.show_about {
            self.show_about = false;
        } else if self.presence.show_pairing {
            self.presence.show_pairing = false;
        }
        cx.notify();
    }

    fn confirm_modal_action(&mut self, cx: &mut Context<Self>) {
        let pending = std::mem::replace(&mut self.modal_pending, ModalPending::None);
        self.modal = None;
        match pending {
            ModalPending::ExplorerDelete(path) => {
                self.explorer
                    .update(cx, |panel, cx| panel.execute_delete(path, cx));
            }
            ModalPending::None => {}
        }
        cx.notify();
    }

    /// Poll the sync engine for events and update presence + console.
    fn poll_sync_events(&mut self, cx: &mut Context<Self>) {
        let Some(ref mut engine) = self.sync_engine else { return };

        let events = engine.tick();
        let channel_events = engine.drain_events();
        let all_events: Vec<SyncEvent> = events.into_iter().chain(channel_events).collect();

        let mut changed = false;
        // Deferred: call refresh_collections once after processing all entries,
        // not once per entry (avoids N expensive filesystem scans per poll tick).
        let mut should_refresh_collections = false;

        for evt in all_events {
            match evt {
                SyncEvent::PeerJoined(peer_id) => {
                    let display_name = format!("Peer-{}", &peer_id[..peer_id.len().min(8)]);
                    self.presence.upsert_peer(peer_id.clone(), display_name, PeerSource::P2P);
                    self.console_panel.update(cx, |panel, cx| {
                        panel.log(
                            ConsoleEntry::team(format!("Peer joined: {}", &peer_id[..peer_id.len().min(8)])),
                            cx,
                        );
                    });
                    changed = true;
                }
                SyncEvent::PeerLeft(peer_id) => {
                    self.presence.remove_peer(&peer_id);
                    self.console_panel.update(cx, |panel, cx| {
                        panel.log(
                            ConsoleEntry::team(format!("Peer left: {}", &peer_id[..peer_id.len().min(8)])),
                            cx,
                        );
                    });
                    changed = true;
                }
                SyncEvent::LiveActivity(activity) => {
                    self.presence.upsert_peer(
                        activity.node_id.clone(),
                        activity.node_name.clone(),
                        PeerSource::UDPBroadcast,
                    );
                    let method = activity.method.clone();
                    let url = activity.url.clone();
                    let status = activity.status;
                    let time_ms = activity.time_ms;
                    self.console_panel.update(cx, |panel, cx| {
                        let msg = if status > 0 {
                            format!("{} {} → {} ({}ms)", method, url, status, time_ms)
                        } else {
                            format!("{} {}", method, url)
                        };
                        panel.log(ConsoleEntry::team(format!("[{}] {}", activity.node_name, msg)), cx);
                    });
                    changed = true;
                }
                SyncEvent::BackendStatus { backend, ready } => {
                    let status = if ready { "online" } else { "offline" };
                    self.console_panel.update(cx, |panel, cx| {
                        panel.log(ConsoleEntry::team(format!("{:?} sync {}", backend, status)), cx);
                    });
                }
                SyncEvent::SyncError(err) => {
                    self.console_panel.update(cx, |panel, cx| {
                        panel.log(ConsoleEntry::team(format!("Sync error: {}", err)), cx);
                    });
                }
                SyncEvent::EntryReceived(entry) => {
                    use protide_core::sync::DataType;
                    if entry.data_type == DataType::Collection || entry.data_type == DataType::Request {
                        // Mark for a single deferred refresh rather than refreshing per-entry.
                        should_refresh_collections = true;
                    }
                    self.console_panel.update(cx, |panel, cx| {
                        panel.log(
                            ConsoleEntry::team(format!("[sync] entry received: {:?}", entry.data_type)),
                            cx,
                        );
                    });
                    changed = true;
                }
                SyncEvent::HandshakeComplete { peer_id, peer_name } => {
                    self.handshake_started = None;
                    self.presence.set_connected(peer_id.clone(), peer_name.clone());
                    self.console_panel.update(cx, |panel, cx| {
                        panel.log(ConsoleEntry::team(format!("Connected to {}", peer_name)), cx);
                    });
                    changed = true;
                }
                SyncEvent::HandshakeFailed { reason } => {
                    self.handshake_started = None;
                    self.presence.connection_status = ConnectionStatus::Error(reason.clone());
                    self.presence.handshake_tick = false;
                    self.console_panel.update(cx, |panel, cx| {
                        panel.log(ConsoleEntry::system(format!("[PAKE] Handshake failed: {}", reason)), cx);
                    });
                    changed = true;
                }
                SyncEvent::P2PDiagnostic(msg) => {
                    self.console_panel.update(cx, |panel, cx| {
                        panel.log(ConsoleEntry::system(msg), cx);
                    });
                }
                SyncEvent::LocalAddr(addr) => {
                    self.console_panel.update(cx, |panel, cx| {
                        panel.log(ConsoleEntry::system(format!("[P2P] Listening on: {}", addr)), cx);
                    });
                }
            }
        }

        // Single filesystem scan for all batched CRDT entries.
        if should_refresh_collections {
            self.explorer.update(cx, |exp, cx| exp.refresh_collections(cx));
        }

        // Tick the handshake pulse so the Connect button border animates at ~1Hz
        if self.presence.connection_status == ConnectionStatus::Handshaking {
            if let Some(started) = self.handshake_started {
                if started.elapsed() > std::time::Duration::from_secs(10) {
                    self.handshake_started = None;
                    self.presence.connection_status =
                        ConnectionStatus::Error("Peer Not Found".to_string());
                    self.presence.handshake_tick = false;
                    self.console_panel.update(cx, |panel, cx| {
                        panel.log(
                            ConsoleEntry::system("[PAKE] Handshake timed out: Peer Not Found"),
                            cx,
                        );
                    });
                    changed = true;
                }
            }
            self.presence.tick_handshake();
            changed = true;
        }

        if changed {
            self.presence.reap_stale();
            // Throttle: skip the redraw if we already notified within the last 100ms.
            // This prevents bursts of CRDT entries or live-probe packets from causing
            // more frame submissions than the GPU can display.
            let now = std::time::Instant::now();
            if now.duration_since(self.last_p2p_notify).as_millis() >= 100 {
                self.last_p2p_notify = now;
                cx.notify();
            }
        }
    }
}

impl Render for MainWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let show_response = self.request_panel.read(cx).has_response_panel();
        let show_codegen = self.request_panel.read(cx).codegen_content.is_some();
        let import_modal: Option<gpui::AnyElement> = if self.request_panel.read(cx).import_modal_open {
            Some(self.request_panel.update(cx, |p, cx| p.render_import_modal(cx)))
        } else {
            None
        };
        let is_dragging = self.drag_sidebar.is_some()
            || self.drag_response.is_some()
            || self.drag_mock_server.is_some()
            || self.drag_codegen.is_some()
            || self.drag_console.is_some();
        let is_col_drag = self.drag_sidebar.is_some()
            || self.drag_mock_server.is_some()
            || self.drag_codegen.is_some();

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.colors.bg_primary)
            .text_color(theme.colors.text_primary)
            .track_focus(&self.focus)
            .key_context("MainWindow")
            .on_action(cx.listener(|this, _: &SendRequest, _, cx| {
                this.request_panel.update(cx, |p, cx| p.send_request(cx));
            }))
            .on_action(cx.listener(|this, _: &SaveRequest, _, cx| {
                this.request_panel.update(cx, |p, cx| p.save_request(cx));
            }))
            .on_action(cx.listener(|this, _: &ToggleSidebar, _, cx| {
                this.toggle_sidebar(cx);
            }))
            .on_action(cx.listener(|this, _: &ToggleMockServer, _, cx| {
                this.toggle_mock_server(cx);
            }))
            .on_action(cx.listener(|this, _: &ToggleConsole, _, cx| {
                this.toggle_console(cx);
            }))
            .on_action(cx.listener(|this, _: &ShowHelp, _, cx| {
                this.show_help = true;
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ShowAbout, _, cx| {
                this.show_about = true;
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &DismissOverlay, _, cx| {
                this.dismiss_overlay(cx);
            }))
            .on_action(|_: &Quit, _, cx: &mut App| {
                cx.quit();
            })
            .child(self.render_title_bar(cx))
            .child(
                div()
                    .flex_1()
                    .flex()
                    .overflow_hidden()
                    // Collapsed sidebar strip
                    .when(self.sidebar_collapsed, |el| {
                        el.child(
                            div()
                                .id("sidebar-collapsed-strip")
                                .w(px(32.0))
                                .h_full()
                                .flex_shrink_0()
                                .bg(theme.colors.bg_secondary)
                                .border_r_1()
                                .border_color(theme.colors.border)
                                .flex()
                                .flex_col()
                                .items_center()
                                .gap(px(2.0))
                                .pt(px(8.0))
                                // Hamburger: expand sidebar
                                .child(
                                    div()
                                        .id("collapse-toggle")
                                        .w(px(28.0))
                                        .h(px(28.0))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .rounded(px(4.0))
                                        .cursor_pointer()
                                        .hover(|s| s.bg(theme.colors.bg_tertiary))
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.toggle_sidebar(cx);
                                        }))
                                        .child(icon(ICON_MENU, ICON_MD, theme.colors.text_muted)),
                                )
                                .child(
                                    div()
                                        .w_full()
                                        .h(px(1.0))
                                        .bg(theme.colors.border)
                                        .mx_auto()
                                        .mt(px(2.0))
                                        .mb(px(2.0)),
                                )
                                // Collections icon
                                .child({
                                    let explorer = self.explorer.clone();
                                    div()
                                        .id("collapsed-collections")
                                        .w(px(28.0))
                                        .h(px(28.0))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .rounded(px(4.0))
                                        .cursor_pointer()
                                        .hover(|s| s.bg(theme.colors.bg_tertiary))
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.toggle_sidebar(cx);
                                            explorer.update(cx, |p, cx| {
                                                p.expand_section_collections(cx)
                                            });
                                        }))
                                        .child(icon(ICON_FOLDER, ICON_MD, theme.colors.text_muted))
                                })
                                // History icon
                                .child({
                                    let explorer = self.explorer.clone();
                                    div()
                                        .id("collapsed-history")
                                        .w(px(28.0))
                                        .h(px(28.0))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .rounded(px(4.0))
                                        .cursor_pointer()
                                        .hover(|s| s.bg(theme.colors.bg_tertiary))
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.toggle_sidebar(cx);
                                            explorer
                                                .update(cx, |p, cx| p.expand_section_history(cx));
                                        }))
                                        .child(icon(ICON_REFRESH, ICON_MD, theme.colors.text_muted))
                                })
                                // Environments icon
                                .child({
                                    let explorer = self.explorer.clone();
                                    div()
                                        .id("collapsed-env")
                                        .w(px(28.0))
                                        .h(px(28.0))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .rounded(px(4.0))
                                        .cursor_pointer()
                                        .hover(|s| s.bg(theme.colors.bg_tertiary))
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.toggle_sidebar(cx);
                                            explorer.update(cx, |p, cx| p.expand_section_env(cx));
                                        }))
                                        .child(icon(
                                            ICON_SETTINGS,
                                            ICON_MD,
                                            theme.colors.text_muted,
                                        ))
                                }),
                        )
                    })
                    // Full sidebar
                    .when(!self.sidebar_collapsed, |el| {
                        el.child(
                            div()
                                .w(px(self.sidebar_width))
                                .h_full()
                                .flex_shrink_0()
                                .bg(theme.colors.bg_secondary)
                                .overflow_hidden()
                                .child(self.explorer.clone()),
                        )
                        // Sidebar resize handle
                        .child(
                            div()
                                .id("sidebar-resize-handle")
                                .w(px(4.0))
                                .h_full()
                                .flex_shrink_0()
                                .border_l_1()
                                .border_color(theme.colors.border)
                                .cursor_col_resize()
                                .hover(|s| s.bg(theme.colors.accent.opacity(0.25)))
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(
                                        |this, event: &gpui::MouseDownEvent, _window, _cx| {
                                            this.drag_sidebar = Some((
                                                f32::from(event.position.x),
                                                this.sidebar_width,
                                            ));
                                        },
                                    ),
                                ),
                        )
                    })
                    // Main content area
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .overflow_hidden()
                            // Request panel: flex_1 in WS/SIO (no response panel), fixed height otherwise
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .w_full()
                                    .overflow_hidden()
                                    .when(show_response, |el| {
                                        el.flex_shrink_0()
                                            .min_h(px(150.0))
                                            .h(px(self.request_height))
                                    })
                                    .when(!show_response, |el| el.flex_1())
                                    .child(self.request_panel.clone()),
                            )
                            // Response resize handle + panel (hidden in WS/SIO modes)
                            .when(show_response, |el| {
                                el
                                    .child(
                                        div()
                                            .id("response-resize-handle")
                                            .w_full()
                                            .h(px(4.0))
                                            .flex_shrink_0()
                                            .border_t_1()
                                            .border_color(theme.colors.border)
                                            .cursor_row_resize()
                                            .hover(|s| s.bg(theme.colors.accent.opacity(0.25)))
                                            .on_mouse_down(
                                                MouseButton::Left,
                                                cx.listener(
                                                    |this, event: &gpui::MouseDownEvent, _window, _cx| {
                                                        this.drag_response = Some((
                                                            f32::from(event.position.y),
                                                            this.request_height,
                                                        ));
                                                    },
                                                ),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_h(px(100.0))
                                            .w_full()
                                            .overflow_hidden()
                                            .child(self.response_panel.clone()),
                                    )
                            })
                            // Console resize handle + panel (toggled with Ctrl+Shift+C)
                            .when(self.show_console, |el| {
                                el
                                    .child(
                                        div()
                                            .id("console-resize-handle")
                                            .w_full()
                                            .h(px(4.0))
                                            .flex_shrink_0()
                                            .border_t_1()
                                            .border_color(theme.colors.border)
                                            .cursor_row_resize()
                                            .hover(|s| s.bg(theme.colors.accent.opacity(0.25)))
                                            .on_mouse_down(
                                                MouseButton::Left,
                                                cx.listener(|this, event: &gpui::MouseDownEvent, _, _| {
                                                    this.drag_console = Some((
                                                        f32::from(event.position.y),
                                                        this.console_height,
                                                    ));
                                                }),
                                            )
                                    )
                                    .child(
                                        div()
                                            .w_full()
                                            .h(px(self.console_height))
                                            .flex_shrink_0()
                                            .overflow_hidden()
                                            .child(self.console_panel.clone()),
                                    )
                            }),
                    )
                    // Mock Server panel (optional right sidebar)
                    .when(self.show_mock_server, |el| {
                        el
                            // Mock server resize handle (left edge)
                            .child(
                                div()
                                    .id("mock-server-resize-handle")
                                    .w(px(4.0))
                                    .h_full()
                                    .flex_shrink_0()
                                    .border_r_1()
                                    .border_color(theme.colors.border)
                                    .cursor_col_resize()
                                    .hover(|s| s.bg(theme.colors.accent.opacity(0.25)))
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(
                                            |this, event: &gpui::MouseDownEvent, _window, _cx| {
                                                this.drag_mock_server = Some((
                                                    f32::from(event.position.x),
                                                    this.mock_server_width,
                                                ));
                                            },
                                        ),
                                    ),
                            )
                            .child(
                                div()
                                    .w(px(self.mock_server_width))
                                    .h_full()
                                    .flex_shrink_0()
                                    .overflow_hidden()
                                    .child(self.mock_server_panel.clone()),
                            )
                    })
                    // Codegen panel (optional right sidebar)
                    .when(show_codegen, |el| {
                        el.child(
                            div()
                                .id("codegen-resize-handle")
                                .w(px(4.0))
                                .h_full()
                                .flex_shrink_0()
                                .border_l_1()
                                .border_color(theme.colors.border)
                                .cursor_col_resize()
                                .hover(|s| s.bg(theme.colors.accent.opacity(0.25)))
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(
                                        |this, event: &gpui::MouseDownEvent, _window, _cx| {
                                            this.drag_codegen = Some((
                                                f32::from(event.position.x),
                                                this.codegen_panel_width,
                                            ));
                                        },
                                    ),
                                ),
                        )
                        .child(self.render_codegen_panel(cx))
                    })
                    // Drag overlay — captures mouse during resize, must be last child
                    .when(is_dragging, |el| {
                        el.child(
                            div()
                                .id("resize-drag-overlay")
                                .absolute()
                                .top_0()
                                .left_0()
                                .w_full()
                                .h_full()
                                .when(is_col_drag, |el| el.cursor_col_resize())
                                .when(!is_col_drag, |el| el.cursor_row_resize())
                                .on_mouse_move(cx.listener(
                                    |this, event: &gpui::MouseMoveEvent, _window, cx| {
                                        let mouse_x = f32::from(event.position.x);
                                        let mouse_y = f32::from(event.position.y);
                                        if let Some((start_x, start_w)) = this.drag_sidebar {
                                            this.sidebar_width =
                                                (start_w + mouse_x - start_x).max(150.0).min(600.0);
                                            cx.notify();
                                        }
                                        if let Some((start_y, start_h)) = this.drag_response {
                                            this.request_height =
                                                (start_h + mouse_y - start_y).max(150.0).min(800.0);
                                            cx.notify();
                                        }
                                        if let Some((start_x, start_w)) = this.drag_mock_server {
                                            // dragging left edge: moving left increases width
                                            this.mock_server_width = (start_w
                                                - (mouse_x - start_x))
                                                .max(200.0)
                                                .min(700.0);
                                            cx.notify();
                                        }
                                        if let Some((start_x, start_w)) = this.drag_codegen {
                                            // dragging left edge: moving left increases width
                                            this.codegen_panel_width = (start_w
                                                - (mouse_x - start_x))
                                                .max(250.0)
                                                .min(800.0);
                                            cx.notify();
                                        }
                                        if let Some((start_y, start_h)) = this.drag_console {
                                            // dragging top edge of console: moving up increases height
                                            this.console_height = (start_h - (mouse_y - start_y))
                                                .max(80.0)
                                                .min(500.0);
                                            cx.notify();
                                        }
                                    },
                                ))
                                .on_mouse_up(
                                    MouseButton::Left,
                                    cx.listener(|this, _, _window, cx| {
                                        if this.drag_sidebar.take().is_some() {
                                            crate::prefs::set_f32(
                                                "main.sidebar_width",
                                                this.sidebar_width,
                                            );
                                        }
                                        if this.drag_response.take().is_some() {
                                            crate::prefs::set_f32(
                                                "main.request_height",
                                                this.request_height,
                                            );
                                        }
                                        if this.drag_mock_server.take().is_some() {
                                            crate::prefs::set_f32(
                                                "main.mock_server_width",
                                                this.mock_server_width,
                                            );
                                        }
                                        if this.drag_codegen.take().is_some() {
                                            crate::prefs::set_f32(
                                                "main.codegen_panel_width",
                                                this.codegen_panel_width,
                                            );
                                        }
                                        this.drag_console.take();
                                        cx.notify();
                                    }),
                                ),
                        )
                    }),
            )
            .child(self.render_status_bar(cx))
            // Full-window modal overlay (always on top)
            .when_some(self.modal.clone(), |el, modal| {
                let theme = theme::current(cx);
                let is_confirm = modal.kind == ModalKind::Confirm;
                let buttons = if is_confirm {
                    div()
                        .flex()
                        .justify_end()
                        .gap(px(8.0))
                        .mt(px(4.0))
                        .child(
                            div()
                                .id("modal-cancel")
                                .px(px(20.0))
                                .py(px(8.0))
                                .bg(theme.colors.bg_tertiary)
                                .border_1()
                                .border_color(theme.colors.border)
                                .text_color(theme.colors.text_secondary)
                                .text_size(px(12.0))
                                .cursor_pointer()
                                .hover(|s| s.bg(theme.colors.bg_elevated))
                                .on_click(cx.listener(|this, _, _, cx| this.dismiss_modal(cx)))
                                .child("Cancel"),
                        )
                        .child(
                            div()
                                .id("modal-confirm")
                                .px(px(20.0))
                                .py(px(8.0))
                                .bg(theme.colors.error)
                                .text_color(theme.colors.bg_primary)
                                .text_size(px(12.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .cursor_pointer()
                                .hover(|s| s.opacity(0.85))
                                .on_click(
                                    cx.listener(|this, _, _, cx| this.confirm_modal_action(cx)),
                                )
                                .child("Delete"),
                        )
                        .into_any_element()
                } else {
                    div()
                        .flex()
                        .justify_end()
                        .mt(px(4.0))
                        .child(
                            div()
                                .id("modal-ok")
                                .px(px(24.0))
                                .py(px(8.0))
                                .bg(theme.colors.accent)
                                .text_color(theme.colors.bg_primary)
                                .text_size(px(12.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .cursor_pointer()
                                .hover(|s| s.bg(theme.colors.accent_hover))
                                .on_click(cx.listener(|this, _, _, cx| this.dismiss_modal(cx)))
                                .child("OK"),
                        )
                        .into_any_element()
                };
                el.child(render_modal_shell(&modal, &theme, buttons))
            })
            // Pairing flyout — anchored below the presence badge (left ~300px, below 40px titlebar)
            .when(self.presence.show_pairing, |el| {
                let flyout = self.render_pairing_flyout_panel(cx);
                let toolbar_h = theme.sizes.toolbar;
                el
                    .child(
                        div()
                            .absolute().top(toolbar_h).left_0().w_full().h_full()
                            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                                this.presence.show_pairing = false;
                                this.presence.reset_connection();
                                cx.notify();
                            }))
                    )
                    .child(
                        gpui::deferred(
                            div()
                                .absolute()
                                .top(px(40.0))
                                .left(px(300.0))
                                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                                .child(flyout)
                        ).with_priority(10)
                    )
            })
            .when(self.open_menu.is_some(), |el| el
                .child(
                    div()
                        .absolute().top_0().left_0().w_full().h_full()
                        .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                            this.open_menu = None;
                            cx.notify();
                        }))
                )
                .child(gpui::deferred(self.render_menu_dropdown(cx)).with_priority(10))
            )
            .when_some(import_modal, |el, modal| el.child(modal))
            .when(self.show_help, |el| el.child(self.render_help_overlay(cx)))
            .when(self.show_about, |el| {
                el.child(self.render_about_overlay(cx))
            })
    }
}

impl MainWindow {
    fn render_title_bar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let show_mock = self.show_mock_server;

        div()
            .id("titlebar")
            .h(theme.sizes.toolbar)
            .w_full()
            .flex()
            .items_center()
            .bg(theme.colors.bg_primary)
            .border_b_1()
            .border_color(theme.colors.border)
            // Logo + title (draggable)
            .child(
                div()
                    .id("titlebar-drag")
                    .flex()
                    .items_center()
                    .gap(px(7.0))
                    .px(px(8.0))
                    .h_full()
                    .cursor_pointer()
                    .on_mouse_down(gpui::MouseButton::Left, |_, window, _cx: &mut App| {
                        window.start_window_move();
                    })
                    // Logo badge
                    .child(
                        div()
                            .size(px(18.0))
                            .bg(theme.colors.accent)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_color(theme.colors.bg_primary)
                                    .child("P"),
                            ),
                    )
                    // Title
                    .child(
                        div()
                            .text_size(px(12.0))
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(theme.colors.text_primary)
                            .child("Protide"),
                    ),
            )
            // Menu bar buttons
            .child({
                let open = self.open_menu;
                let menus: &[(u8, &str)] = &[(0, "Protide"), (1, "Request"), (2, "View"), (3, "Help")];
                div()
                    .flex().items_center().h_full()
                    .children(menus.iter().map(|&(id, label)| {
                        let is_open = open == Some(id);
                        div()
                            .id(("menu-btn", id as usize))
                            .h_full().px(px(10.0))
                            .flex().items_center()
                            .cursor_pointer()
                            .text_size(px(12.0))
                            .when(is_open, |el| el
                                .bg(theme.colors.bg_tertiary)
                                .text_color(theme.colors.text_primary)
                            )
                            .when(!is_open, |el| el
                                .text_color(theme.colors.text_secondary)
                                .hover(|s| s.bg(theme.colors.bg_elevated).text_color(theme.colors.text_primary))
                            )
                            .child(label)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.open_menu = if this.open_menu == Some(id) { None } else { Some(id) };
                                cx.notify();
                            }))
                    }))
            })
            // Presence bar (collaboration) — click to toggle pairing flyout
            .child(
                div()
                    .id("presence-bar")
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.presence.show_pairing = !this.presence.show_pairing;
                        cx.notify();
                    }))
                    .child(self.presence.render_presence_bar(&theme))
            )
            // Drag region (fills remaining space)
            .child(div().flex_1().h_full().on_mouse_down(
                gpui::MouseButton::Left,
                |_, window, _cx: &mut App| {
                    window.start_window_move();
                },
            ))
            // Mock server toggle
            .child(
                div()
                    .id("btn-mock-server")
                    .h(px(22.0))
                    .px(px(8.0))
                    .mr(px(6.0))
                    .flex()
                    .items_center()
                    .cursor_pointer()
                    .bg(if show_mock {
                        theme.colors.accent.opacity(0.15)
                    } else {
                        theme.colors.bg_elevated
                    })
                    .border_1()
                    .border_color(if show_mock {
                        theme.colors.accent.opacity(0.4)
                    } else {
                        theme.colors.border
                    })
                    .hover(|s| s.border_color(theme.colors.accent.opacity(0.5)))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.toggle_mock_server(cx);
                    }))
                    .child(
                        div()
                            .text_size(px(10.0))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(if show_mock {
                                theme.colors.accent
                            } else {
                                theme.colors.text_secondary
                            })
                            .child("Mock Server"),
                    ),
            )
            // Window controls
            .child(
                div()
                    .flex()
                    .items_center()
                    .h_full()
                    .border_l_1()
                    .border_color(theme.colors.border)
                    .child(
                        div()
                            .id("btn-minimize")
                            .w(px(36.0))
                            .h_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .text_color(theme.colors.text_secondary)
                            .hover(|s| s.bg(theme.colors.bg_elevated))
                            .on_click(|_, window, _cx: &mut App| {
                                window.minimize_window();
                            })
                            .child(icon(ICON_MINIMIZE, 12.0, theme.colors.text_secondary)),
                    )
                    .child(
                        div()
                            .id("btn-maximize")
                            .w(px(36.0))
                            .h_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .text_color(theme.colors.text_secondary)
                            .hover(|s| s.bg(theme.colors.bg_elevated))
                            .on_click(|_, window, _cx: &mut App| {
                                window.toggle_fullscreen();
                            })
                            .child(icon(ICON_MAXIMIZE, 12.0, theme.colors.text_secondary)),
                    )
                    .child(
                        div()
                            .id("btn-close")
                            .w(px(36.0))
                            .h_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .text_color(theme.colors.text_secondary)
                            .hover(|s| s.bg(theme.colors.error).text_color(theme.colors.bg_primary))
                            .on_click(|_, _window, cx: &mut App| {
                                cx.quit();
                            })
                            .child(icon(ICON_WINDOW_CLOSE, 12.0, theme.colors.text_secondary)),
                    ),
            )
    }

    fn render_status_bar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);

        // Read protocol from request panel
        let protocol = self.request_panel.read(cx).mode_label();
        let protocol_color = theme.method_color(protocol);

        // Read last response summary
        let response_info = self.response_panel.read(cx).last_response_summary();
        let is_loading = self.response_panel.read(cx).is_loading();

        let sep = || {
            div()
                .w(px(1.0))
                .h(px(10.0))
                .bg(theme.colors.border)
                .mx(px(6.0))
        };

        div()
            .id("status-bar")
            .h(px(22.0))
            .w_full()
            .flex()
            .items_center()
            .flex_shrink_0()
            .px(px(10.0))
            .gap(px(0.0))
            .bg(theme.colors.bg_primary)
            .border_t_1()
            .border_color(theme.colors.border)
            // Active env dot + label
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(5.0))
                    .child(div().size(px(6.0)).bg(theme.colors.accent))
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(theme.colors.text_secondary)
                            .child("Local Dev"),
                    ),
            )
            .child(sep())
            // Protocol badge
            .child(
                div()
                    .text_size(px(10.0))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(protocol_color)
                    .child(protocol),
            )
            .child(sep())
            // Response info or ready state
            .child(if is_loading {
                div()
                    .flex()
                    .items_center()
                    .gap(px(5.0))
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(theme.colors.text_muted)
                            .child("Sending…"),
                    )
                    .into_any_element()
            } else if let Some((status, _, time_ms, size_bytes)) = response_info {
                let status_color = theme.status_color(status);
                let size_str = if size_bytes >= 1024 * 1024 {
                    format!("{:.1} MB", size_bytes as f64 / (1024.0 * 1024.0))
                } else if size_bytes >= 1024 {
                    format!("{:.1} KB", size_bytes as f64 / 1024.0)
                } else {
                    format!("{} B", size_bytes)
                };
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .child(
                        div()
                            .text_size(px(10.0))
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(status_color)
                            .child(format!("{}", status)),
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(theme.colors.text_muted)
                            .child("·"),
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(theme.colors.text_secondary)
                            .child(format!("{}ms", time_ms)),
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(theme.colors.text_muted)
                            .child("·"),
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(theme.colors.text_secondary)
                            .child(size_str),
                    )
                    .into_any_element()
            } else {
                div()
                    .text_size(px(10.0))
                    .text_color(theme.colors.text_muted)
                    .child("Ready")
                    .into_any_element()
            })
            .child(div().flex_1())
            // Console toggle (right side of status bar)
            .child({
                let show_console = self.show_console;
                let count = self.console_panel.read(cx).entry_count();
                div()
                    .id("toggle-console-btn")
                    .h_full()
                    .px(px(8.0))
                    .flex()
                    .items_center()
                    .gap(px(5.0))
                    .cursor_pointer()
                    .when(show_console, |el| el.bg(theme.colors.accent.opacity(0.12)))
                    .hover(|s| s.bg(theme.colors.bg_tertiary))
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_console(cx)))
                    .child(
                        div()
                            .text_size(px(10.0))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(if show_console {
                                theme.colors.accent
                            } else {
                                theme.colors.text_muted
                            })
                            .child("Console")
                    )
                    .when(count > 0, |el| {
                        el.child(
                            div()
                                .px(px(4.0))
                                .py(px(1.0))
                                .bg(theme.colors.accent.opacity(0.15))
                                .text_size(px(9.0))
                                .text_color(theme.colors.accent)
                                .child(SharedString::from(format!("{}", count)))
                        )
                    })
            })
    }

    fn render_codegen_panel(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let panel = self.request_panel.read(cx);
        let editor = panel.codegen_editor.clone();
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
            // Header
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
                    // Language tabs
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
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.request_panel
                                        .update(cx, |panel, cx| panel.generate_code(lang, cx));
                                }))
                                .child(label)
                        }),
                    ))
                    // Copy button
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
                    // Close button
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
            // Code editor
            .child(div().flex_1().overflow_hidden().child(editor))
    }

    fn render_menu_dropdown(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let toolbar_h = 40.0f32;

        // (label, shortcut_hint, action_fn)
        type ActionFn = Box<dyn Fn(&mut gpui::Window, &mut gpui::App)>;
        let items: Vec<(&str, &str, ActionFn)> = match self.open_menu {
            Some(0) => vec![
                ("About Protide",     "",          Box::new(|w, cx| w.dispatch_action(Box::new(ShowAbout), cx))),
                ("---",               "",          Box::new(|_, _| {})),
                ("Quit",              "Ctrl+Q",    Box::new(|w, cx| w.dispatch_action(Box::new(Quit), cx))),
            ],
            Some(1) => vec![
                ("Send Request",      "Ctrl+Enter", Box::new(|w, cx| w.dispatch_action(Box::new(SendRequest), cx))),
                ("Save Request",      "Ctrl+S",     Box::new(|w, cx| w.dispatch_action(Box::new(SaveRequest), cx))),
            ],
            Some(2) => vec![
                ("Toggle Sidebar",    "Ctrl+B",     Box::new(|w, cx| w.dispatch_action(Box::new(ToggleSidebar), cx))),
                ("Toggle Mock Server","Ctrl+Shift+M",Box::new(|w, cx| w.dispatch_action(Box::new(ToggleMockServer), cx))),
            ],
            Some(3) => vec![
                ("Keyboard Shortcuts","F1",          Box::new(|w, cx| w.dispatch_action(Box::new(ShowHelp), cx))),
            ],
            _ => vec![],
        };

        // Horizontal offset per menu id (approximate, based on title bar layout)
        let left_px = match self.open_menu {
            Some(0) => 88.0,
            Some(1) => 148.0,
            Some(2) => 220.0,
            Some(3) => 272.0,
            _       => 88.0,
        };

        div()
            .id("menu-dropdown")
            .absolute()
            .top(px(toolbar_h))
            .left(px(left_px))
            .min_w(px(200.0))
            .py(px(4.0))
            .bg(theme.colors.bg_elevated)
            .border_1()
            .border_color(theme.colors.border)
            .shadow_lg()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .children(items.into_iter().enumerate().map(|(i, (label, hint, action))| {
                if label == "---" {
                    return div()
                        .id(("menu-sep", i))
                        .my(px(3.0))
                        .mx(px(6.0))
                        .h(px(1.0))
                        .bg(theme.colors.border)
                        .into_any_element();
                }
                div()
                    .id(("menu-item", i))
                    .px(px(12.0)).py(px(7.0))
                    .flex().items_center().justify_between()
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.colors.bg_tertiary))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.open_menu = None;
                        cx.notify();
                        action(window, cx);
                    }))
                    .child(
                        div().text_size(px(12.0)).text_color(theme.colors.text_primary).child(label)
                    )
                    .when(!hint.is_empty(), |el| el.child(
                        div().text_size(px(10.0)).text_color(theme.colors.text_muted).ml(px(24.0)).child(hint)
                    ))
                    .into_any_element()
            }))
    }

    fn render_help_overlay(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);

        let shortcuts: &[(&str, &str, &str)] = &[
            ("Request", "Ctrl+Enter", "Send request"),
            ("Request", "Ctrl+S", "Save request"),
            ("View", "Ctrl+B", "Toggle sidebar"),
            ("View", "Ctrl+Shift+M", "Toggle mock server"),
            ("Help", "F1", "Show keyboard shortcuts"),
            ("Help", "Ctrl+Shift+A", "About Protide"),
            ("General", "Escape", "Close dialog / overlay"),
        ];

        div()
            .absolute()
            .top_0()
            .left_0()
            .w_full()
            .h_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(theme.colors.bg_primary.opacity(0.7))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.show_help = false;
                    cx.notify();
                }),
            )
            .child(
                div()
                    .w(px(480.0))
                    .bg(theme.colors.bg_elevated)
                    .border_1()
                    .border_color(theme.colors.border)
                    .shadow_lg()
                    .on_mouse_down(MouseButton::Left, |_, _, _| {}) // stop propagation
                    .child(
                        // Header
                        div()
                            .px(px(20.0))
                            .py(px(14.0))
                            .border_b_1()
                            .border_color(theme.colors.border)
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_size(px(14.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(theme.colors.text_primary)
                                    .child("Keyboard Shortcuts"),
                            )
                            .child(
                                div()
                                    .id("help-close")
                                    .px(px(8.0))
                                    .py(px(4.0))
                                    .text_size(px(11.0))
                                    .text_color(theme.colors.text_muted)
                                    .cursor_pointer()
                                    .hover(|s| s.text_color(theme.colors.text_primary))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.show_help = false;
                                        cx.notify();
                                    }))
                                    .child("✕"),
                            ),
                    )
                    .child(
                        // Shortcut rows
                        div()
                            .px(px(20.0))
                            .py(px(12.0))
                            .flex()
                            .flex_col()
                            .gap(px(2.0))
                            .children(shortcuts.iter().map(|(group, key, desc)| {
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .py(px(5.0))
                                    .border_b_1()
                                    .border_color(theme.colors.border.opacity(0.4))
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(10.0))
                                            .child(
                                                div()
                                                    .w(px(60.0))
                                                    .text_size(px(10.0))
                                                    .text_color(theme.colors.text_muted)
                                                    .child(*group),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(12.0))
                                                    .text_color(theme.colors.text_secondary)
                                                    .child(*desc),
                                            ),
                                    )
                                    .child(div().flex().items_center().gap(px(3.0)).children(
                                        key.split('+').map(|k| {
                                            div()
                                                .px(px(7.0))
                                                .py(px(3.0))
                                                .bg(theme.colors.bg_tertiary)
                                                .border_1()
                                                .border_color(theme.colors.border)
                                                .text_size(px(10.0))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(theme.colors.text_primary)
                                                .child(k.trim())
                                        }),
                                    ))
                            })),
                    )
                    .child(
                        div()
                            .px(px(20.0))
                            .py(px(10.0))
                            .border_t_1()
                            .border_color(theme.colors.border)
                            .text_size(px(10.0))
                            .text_color(theme.colors.text_muted)
                            .child("Press Escape or click outside to close"),
                    ),
            )
    }

    fn render_about_overlay(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);

        div()
            .absolute()
            .top_0()
            .left_0()
            .w_full()
            .h_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(theme.colors.bg_primary.opacity(0.7))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.show_about = false;
                    cx.notify();
                }),
            )
            .child(
                div()
                    .w(px(360.0))
                    .bg(theme.colors.bg_elevated)
                    .border_1()
                    .border_color(theme.colors.border)
                    .shadow_lg()
                    .on_mouse_down(MouseButton::Left, |_, _, _| {})
                    .child(
                        // Header with close
                        div()
                            .px(px(20.0))
                            .py(px(14.0))
                            .border_b_1()
                            .border_color(theme.colors.border)
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_size(px(13.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(theme.colors.text_primary)
                                    .child("About"),
                            )
                            .child(
                                div()
                                    .id("about-close")
                                    .px(px(8.0))
                                    .py(px(4.0))
                                    .text_size(px(11.0))
                                    .text_color(theme.colors.text_muted)
                                    .cursor_pointer()
                                    .hover(|s| s.text_color(theme.colors.text_primary))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.show_about = false;
                                        cx.notify();
                                    }))
                                    .child("✕"),
                            ),
                    )
                    .child(
                        div()
                            .px(px(28.0))
                            .py(px(24.0))
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap(px(14.0))
                            // Logo badge
                            .child(
                                div()
                                    .size(px(56.0))
                                    .bg(theme.colors.accent)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(
                                        div()
                                            .text_size(px(28.0))
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(theme.colors.bg_primary)
                                            .child("P"),
                                    ),
                            )
                            // Name + version
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .items_center()
                                    .gap(px(4.0))
                                    .child(
                                        div()
                                            .text_size(px(20.0))
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(theme.colors.text_primary)
                                            .child("Protide"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.0))
                                            .text_color(theme.colors.text_muted)
                                            .child(format!(
                                                "Version {}",
                                                env!("CARGO_PKG_VERSION")
                                            )),
                                    ),
                            )
                            // Description
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(theme.colors.text_secondary)
                                    .text_center()
                                    .child("Free and open-source API testing tool"),
                            )
                            // Divider
                            .child(div().w_full().h(px(1.0)).bg(theme.colors.border))
                            // Developer
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .items_center()
                                    .gap(px(3.0))
                                    .child(
                                        div()
                                            .text_size(px(11.0))
                                            .text_color(theme.colors.text_muted)
                                            .child("Developed by"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(13.0))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(theme.colors.text_primary)
                                            .child("Rakibul Yeasin"),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .px(px(20.0))
                            .py(px(10.0))
                            .border_t_1()
                            .border_color(theme.colors.border)
                            .flex()
                            .justify_center()
                            .child(
                                div()
                                    .id("about-ok")
                                    .px(px(28.0))
                                    .py(px(7.0))
                                    .bg(theme.colors.accent)
                                    .text_color(theme.colors.bg_primary)
                                    .text_size(px(12.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .cursor_pointer()
                                    .hover(|s| s.bg(theme.colors.accent_hover))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.show_about = false;
                                        cx.notify();
                                    }))
                                    .child("Close"),
                            ),
                    ),
            )
    }
}
