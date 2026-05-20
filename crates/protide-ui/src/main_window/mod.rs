//! Main application window

mod codegen;
mod drag;
mod help;
mod overlays;
mod render;
mod render_sidebar;
mod statusbar;
mod sync;
mod titlebar;

use std::path::PathBuf;

use gpui::{Context, Entity, FocusHandle, KeyBinding, WeakEntity, Window, prelude::*};

gpui::actions!(
    main_window,
    [
        SendRequest,
        SaveRequest,
        ToggleSidebar,
        ToggleMockServer,
        ToggleConsole,
        ToggleDocs,
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
        KeyBinding::new("ctrl-shift-d", ToggleDocs, None),
        KeyBinding::new("f1", ShowHelp, None),
        KeyBinding::new("ctrl-shift-a", ShowAbout, None),
        KeyBinding::new("escape", DismissOverlay, None),
        KeyBinding::new("ctrl-q", Quit, None),
    ]);
}

use super::panels::presence::{ConnectionStatus, PeerSource, PresenceManager};
use super::components::{TextInput, TextInputStyle};
use super::panels::{ConsoleEntry, ConsolePanel, DocsPanel, ExplorerPanel, MockServerPanel, RequestPanel, ResponsePanel, RunnerPanel};
use crate::theme;
use protide_core::sync::{SyncEngine, SyncEvent};
use crate::components::icons::{
    ICON_CLOSE, ICON_COPY, ICON_FOLDER, ICON_MAXIMIZE, ICON_MD, ICON_MENU, ICON_MINIMIZE,
    ICON_REFRESH, ICON_SETTINGS, ICON_SM, ICON_WINDOW_CLOSE, icon,
};
use crate::components::modal::{ModalKind, ModalState, render_modal_shell};

/// Pending action for confirm modals
#[derive(Clone, Debug, Default)]
pub(super) enum ModalPending {
    #[default]
    None,
    ExplorerDelete(PathBuf),
}

/// Main window containing the application layout
pub struct MainWindow {
    pub(super) explorer: Entity<ExplorerPanel>,
    pub(super) runner_panel: Entity<RunnerPanel>,
    pub(super) show_runner: bool,
    pub(super) request_panel: Entity<RequestPanel>,
    pub(super) response_panel: Entity<ResponsePanel>,
    pub(super) mock_server_panel: Entity<MockServerPanel>,
    pub(super) console_panel: Entity<ConsolePanel>,
    pub(super) docs_panel: Entity<DocsPanel>,
    pub(super) show_console: bool,
    pub(super) console_height: f32,
    pub(super) drag_console: Option<(f32, f32)>,
    pub(super) show_mock_server: bool,
    pub(super) show_docs: bool,
    pub(super) docs_width: f32,
    pub(super) drag_docs: Option<(f32, f32)>,
    pub(super) sidebar_collapsed: bool,
    pub(super) sidebar_width: f32,
    pub(super) request_height: f32,
    pub(super) mock_server_width: f32,
    pub(super) codegen_panel_width: f32,
    pub(super) drag_sidebar: Option<(f32, f32)>,
    pub(super) drag_response: Option<(f32, f32)>,
    pub(super) drag_mock_server: Option<(f32, f32)>,
    pub(super) drag_codegen: Option<(f32, f32)>,
    pub(super) modal: Option<ModalState>,
    pub(super) modal_pending: ModalPending,
    pub(super) show_help: bool,
    pub(super) show_about: bool,
    pub(super) focus: FocusHandle,
    /// Which title-bar menu is open (0=Protide, 1=Request, 2=View, 3=Help)
    pub(super) open_menu: Option<u8>,
    /// Keeps the on_app_quit subscription alive for the lifetime of the window.
    pub(super) _quit_sub: gpui::Subscription,
    /// Collaboration presence manager
    pub(super) presence: PresenceManager,
    /// Sync engine for peer discovery and CRDT sync
    pub(super) sync_engine: Option<SyncEngine>,
    /// Text input for the "Join Peer" pairing code field
    pub(super) join_input: Entity<TextInput>,
    /// When the current PAKE handshake was initiated (for 10-second timeout)
    pub(super) handshake_started: Option<std::time::Instant>,
    /// Last time cx.notify() was called from the P2P sync poller
    pub(super) last_p2p_notify: std::time::Instant,
}

impl MainWindow {
    pub fn build(_window: &mut Window, cx: &mut Context<Self>, sync_engine: Option<SyncEngine>) -> Self {
        let main_window_weak: WeakEntity<MainWindow> = cx.entity().downgrade();
        let explorer = cx.new(|cx| ExplorerPanel::new(cx, main_window_weak.clone()));
        let runner_panel = cx.new(|cx| RunnerPanel::new(cx, main_window_weak.clone()));
        let response_panel = cx.new(ResponsePanel::new);
        let response_panel_clone = response_panel.clone();
        let request_panel = cx.new(|cx| RequestPanel::new(cx, response_panel_clone));
        let mock_server_panel = cx.new(|cx| MockServerPanel::new(cx, main_window_weak));
        let console_panel = cx.new(ConsolePanel::new);
        let docs_panel = cx.new(|_| DocsPanel::new());

        let request_panel_clone = request_panel.clone();
        explorer.update(cx, |explorer, cx| {
            explorer.set_request_panel(request_panel_clone, cx);
        });

        let explorer_clone = explorer.clone();
        request_panel.update(cx, |panel, cx| {
            panel.set_explorer_panel(explorer_clone, cx);
        });

        let explorer_clone = explorer.clone();
        docs_panel.update(cx, |panel, _| {
            panel.set_explorer(explorer_clone);
        });

        let console_clone = console_panel.clone();
        request_panel.update(cx, |panel, cx| {
            panel.set_console_panel(console_clone, cx);
        });

        let session = crate::session::load();
        if let Some(workspace) = session.current_workspace.clone().filter(|p| p.is_dir()) {
            let ws_key = workspace.to_string_lossy().to_string();
            let saved_entry = session.workspaces.get(&ws_key).cloned();
            explorer.update(cx, |exp, cx| {
                exp.init_workspace(workspace.clone(), saved_entry, cx);
            });
        }

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
        if let Some(code) = sync_engine.as_ref().and_then(|e| e.config().pairing_code.as_deref()) {
            let s = gpui::SharedString::from(code.to_string());
            presence.pairing_code = s.clone();
            presence.generated_code = s;
        }

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
            runner_panel,
            show_runner: false,
            request_panel,
            response_panel,
            mock_server_panel,
            console_panel,
            docs_panel,
            show_console: false,
            console_height: 160.0,
            drag_console: None,
            show_mock_server: false,
            show_docs: false,
            docs_width: crate::prefs::get_f32("main.docs_width", 420.0),
            drag_docs: None,
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

    pub fn open_runner(
        &mut self,
        collection_path: PathBuf,
        env_vars: std::collections::HashMap<String, String>,
        cx: &mut Context<Self>,
    ) {
        self.show_runner = true;
        self.runner_panel.update(cx, |panel, cx| {
            panel.start(collection_path, env_vars, cx);
        });
        cx.notify();
    }

    pub fn close_runner(&mut self, cx: &mut Context<Self>) {
        self.show_runner = false;
        cx.notify();
    }

    /// Forward a local workspace file change to the sync engine for P2P broadcast.
    pub fn broadcast_workspace_file(&mut self, workspace_root: &std::path::Path, file_path: &std::path::Path, content: String, deleted: bool) {
        if let Some(ref mut engine) = self.sync_engine {
            engine.broadcast_workspace_file(workspace_root, file_path, content, deleted);
        }
    }

    /// Called by ExplorerPanel when the workspace root changes.
    pub(super) fn toggle_console(&mut self, cx: &mut Context<Self>) {
        self.show_console = !self.show_console;
        cx.notify();
    }

    pub(super) fn capture_session(&self, cx: &Context<Self>) -> crate::session::SessionState {
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

    pub fn ensure_sidebar_visible(&mut self, cx: &mut Context<Self>) {
        if self.sidebar_collapsed {
            self.sidebar_collapsed = false;
            cx.notify();
        }
    }

    pub(super) fn toggle_mock_server(&mut self, cx: &mut Context<Self>) {
        self.show_mock_server = !self.show_mock_server;
        cx.notify();
    }

    pub(super) fn toggle_docs(&mut self, cx: &mut Context<Self>) {
        self.show_docs = !self.show_docs;
        cx.notify();
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

    pub(super) fn dismiss_modal(&mut self, cx: &mut Context<Self>) {
        self.modal = None;
        self.modal_pending = ModalPending::None;
        cx.notify();
    }

    pub(super) fn dismiss_overlay(&mut self, cx: &mut Context<Self>) {
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

    pub(super) fn confirm_modal_action(&mut self, cx: &mut Context<Self>) {
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
}
