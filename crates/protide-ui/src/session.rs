//! Session persistence - saves and restores workspace state across app restarts.
//! File: ~/.config/protide/session.json

use std::collections::HashMap;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

/// Top-level session saved to disk.
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct SessionState {
    /// Workspace that was open when the app last closed.
    pub current_workspace: Option<PathBuf>,
    /// Per-workspace entries, keyed by the workspace directory path (as a string).
    pub workspaces: HashMap<String, WorkspaceEntry>,
}

/// State captured for a single open workspace.
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct WorkspaceEntry {
    /// The .http file that was active when this workspace was last visited.
    pub active_file: Option<PathBuf>,
    /// Editor state at the time of the last save (may include unsaved edits).
    pub draft: Option<RequestDraft>,
    /// Which tree folders were expanded.
    pub expanded_folders: Vec<PathBuf>,
    /// Name of the active environment (used to re-select on restore).
    pub active_env: Option<String>,
}

/// All request-editor state that can be serialised and restored.
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct RequestDraft {
    // ── Protocol & navigation ─────────────────────────────────────────────────
    /// "http" | "graphql" | "websocket" | "grpc" | "trpc" | "socketio"
    pub protocol: String,
    pub active_tab: usize,

    // ── HTTP common ───────────────────────────────────────────────────────────
    pub url: String,
    pub method: String,
    pub headers: Vec<HeaderEntry>,
    pub body: String,
    /// "json" | "xml" | "raw" | "form" | "binary"
    pub body_type: String,

    // ── Auth ──────────────────────────────────────────────────────────────────
    /// "none" | "bearer" | "basic" | "apikey"
    pub auth_type: String,
    pub bearer_token: String,
    pub basic_username: String,
    pub basic_password: String,
    pub api_key_name: String,
    pub api_key_value: String,
    /// "header" | "query"
    pub api_key_location: String,

    // ── GraphQL ───────────────────────────────────────────────────────────────
    pub graphql_query: String,
    pub graphql_variables: String,
    pub graphql_operation_name: String,

    // ── gRPC ─────────────────────────────────────────────────────────────────
    pub grpc_message: String,
    pub grpc_proto_path: Option<PathBuf>,
    pub grpc_service: Option<String>,
    pub grpc_method_name: Option<String>,

    // ── tRPC ─────────────────────────────────────────────────────────────────
    pub trpc_procedure: String,
    pub trpc_params: String,

    // ── Socket.IO ────────────────────────────────────────────────────────────
    pub sio_namespace: String,
    pub sio_event_name: String,
    pub sio_payload: String,
}

/// A single header row captured in the draft.
#[derive(Serialize, Deserialize, Default, Clone)]
pub struct HeaderEntry {
    pub key: String,
    pub value: String,
    pub enabled: bool,
}

// ── Disk I/O ──────────────────────────────────────────────────────────────────

fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("protide").join("session.json"))
}

/// Load session from disk, or return an empty default if the file doesn't exist.
pub fn load() -> SessionState {
    let path = match config_path() {
        Some(p) => p,
        None => return SessionState::default(),
    };
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(_) => return SessionState::default(),
    };
    serde_json::from_str(&text).unwrap_or_default()
}

/// Write session to disk synchronously. Use before app exit where blocking is fine.
pub fn save_sync(state: &SessionState) {
    let Some(path) = config_path() else { return };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(text) = serde_json::to_string(state) {
        let _ = std::fs::write(&path, text);
    }
}

/// Spawn a background thread to write the session so the UI thread isn't blocked.
pub fn save_bg(state: SessionState) {
    std::thread::spawn(move || save_sync(&state));
}
