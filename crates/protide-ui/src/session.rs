//! Session persistence - saves and restores workspace state across app restarts.
//! File: ~/.config/protide/session.json

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use log::warn;
use protide_core::sync::GroupSecret;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const KEYCHAIN_SERVICE: &str = "protide";

type SecretSlot = fn(&mut RequestDraft) -> &mut String;

/// Draft fields kept in the keychain instead of session.json.
const SECRET_FIELDS: [(&str, SecretSlot); 3] = [
    ("bearer_token", |d| &mut d.bearer_token),
    ("basic_password", |d| &mut d.basic_password),
    ("api_key_value", |d| &mut d.api_key_value),
];

/// Top-level session saved to disk.
///
/// Every struct here is `#[serde(default)]`: a session file written by a
/// different build of Protide is missing whatever fields that build did not
/// know about, and without `default` serde rejects the *whole* document,
/// throwing away the user's entire restorable state over one absent field.
#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(default)]
pub struct SessionState {
    /// Workspace that was open when the app last closed.
    pub current_workspace: Option<PathBuf>,
    /// Per-workspace entries, keyed by the workspace directory path (as a string).
    pub workspaces: HashMap<String, WorkspaceEntry>,
}

/// State captured for a single open workspace.
#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(default)]
pub struct WorkspaceEntry {
    /// The .http file that was active when this workspace was last visited.
    pub active_file: Option<PathBuf>,
    /// Which request of `active_file` was loaded, counted from 0.
    pub active_request: usize,
    /// Editor state at the time of the last save (may include unsaved edits).
    pub draft: Option<RequestDraft>,
    /// Which tree folders were expanded.
    pub expanded_folders: Vec<PathBuf>,
    /// Name of the active environment (used to re-select on restore).
    pub active_env: Option<String>,
}

/// All request-editor state that can be serialised and restored.
#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(default)]
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
    /// Secret fields are read from older session files (so they migrate) but
    /// never written back: their values live in the OS keychain.
    #[serde(skip_serializing)]
    pub bearer_token: String,
    pub basic_username: String,
    #[serde(skip_serializing)]
    pub basic_password: String,
    pub api_key_name: String,
    #[serde(skip_serializing)]
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
#[serde(default)]
pub struct HeaderEntry {
    pub key: String,
    pub value: String,
    pub enabled: bool,
}

// ── Disk I/O ──────────────────────────────────────────────────────────────────

fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("protide").join("session.json"))
}

/// Read the session at `file`. A missing, unreadable or malformed file yields
/// the default session - never an error and never a panic.
fn load_from(file: &Path) -> SessionState {
    let text = match std::fs::read_to_string(file) {
        Ok(t) => t,
        Err(_) => return SessionState::default(),
    };
    serde_json::from_str(&text).unwrap_or_default()
}

fn save_to(file: &Path, state: &SessionState) {
    if let Some(dir) = file.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(text) = serde_json::to_string(state) {
        let _ = std::fs::write(file, text);
    }
}

/// Where draft secrets live. A seam so tests never touch the real keychain.
trait SecretStore {
    /// The stored value, or `None` when absent or the store failed.
    fn get(&self, key: &str) -> Option<String>;
    /// Store `value`; an empty value removes the entry.
    fn set(&self, key: &str, value: &str);
}

/// The OS keychain. Failures are logged and treated as "no secret" so a
/// broken keychain never blocks session restore.
struct Keychain;

impl SecretStore for Keychain {
    fn get(&self, key: &str) -> Option<String> {
        match keyring::Entry::new(KEYCHAIN_SERVICE, key).and_then(|e| e.get_password()) {
            Ok(value) => Some(value),
            Err(keyring::Error::NoEntry) => None,
            Err(e) => {
                warn!("Keychain read failed for {key}: {e}");
                None
            }
        }
    }

    fn set(&self, key: &str, value: &str) {
        let result = keyring::Entry::new(KEYCHAIN_SERVICE, key).and_then(|e| {
            if value.is_empty() {
                e.delete_credential()
            } else {
                e.set_password(value)
            }
        });
        match result {
            Ok(()) | Err(keyring::Error::NoEntry) => {}
            Err(e) => warn!("Keychain write failed for {key}: {e}"),
        }
    }
}

/// Visit every draft secret as (keychain key, value). Keys combine the
/// workspace path and the field name.
fn for_each_secret(state: &mut SessionState, mut visit: impl FnMut(String, &mut String)) {
    for (workspace, entry) in &mut state.workspaces {
        if let Some(draft) = entry.draft.as_mut() {
            for (field, slot) in SECRET_FIELDS {
                visit(format!("{workspace}#{field}"), slot(draft));
            }
        }
    }
}

/// Fill secrets from `store`. A value still present (an older session file
/// written before the keychain) wins, so it migrates on the next save.
fn hydrate(state: &mut SessionState, store: &dyn SecretStore) {
    for_each_secret(state, |key, value| {
        if value.is_empty()
            && let Some(secret) = store.get(&key)
        {
            *value = secret;
        }
    });
}

fn stash(state: &mut SessionState, store: &dyn SecretStore) {
    for_each_secret(state, |key, value| store.set(&key, value));
}

/// Keychain entry holding this node's sync group secret. Workspace keys are
/// absolute paths, so this never collides with a draft secret.
const GROUP_SECRET_KEY: &str = "sync#group_secret";

fn read_group_secret(store: &dyn SecretStore) -> Option<GroupSecret> {
    let bytes = BASE64.decode(store.get(GROUP_SECRET_KEY)?).ok()?;
    <[u8; 32]>::try_from(bytes).ok().map(GroupSecret)
}

fn write_group_secret(store: &dyn SecretStore, secret: &GroupSecret) {
    store.set(GROUP_SECRET_KEY, &BASE64.encode(secret.0));
}

/// The persisted sync group secret, so a restart rejoins the same group.
pub fn load_group_secret() -> Option<GroupSecret> {
    read_group_secret(&Keychain)
}

/// Persist the sync group secret in the OS keychain, never in a file. A
/// keychain failure is logged and the engine keeps the secret in memory.
pub fn save_group_secret(secret: &GroupSecret) {
    write_group_secret(&Keychain, secret);
}

/// Load session from disk, or return an empty default if the file doesn't exist.
pub fn load() -> SessionState {
    let mut state = config_path().map(|p| load_from(&p)).unwrap_or_default();
    hydrate(&mut state, &Keychain);
    state
}

/// Write session to disk synchronously. Use before app exit where blocking is fine.
pub fn save_sync(state: &SessionState) {
    let Some(path) = config_path() else { return };
    stash(&mut state.clone(), &Keychain);
    save_to(&path, state);
}

/// Spawn a background thread to write the session so the UI thread isn't blocked.
pub fn save_bg(state: SessionState) {
    std::thread::spawn(move || save_sync(&state));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;
    use std::cell::RefCell;

    #[derive(Default)]
    struct MockStore(RefCell<HashMap<String, String>>);

    impl SecretStore for MockStore {
        fn get(&self, key: &str) -> Option<String> {
            self.0.borrow().get(key).cloned()
        }

        fn set(&self, key: &str, value: &str) {
            if value.is_empty() {
                self.0.borrow_mut().remove(key);
            } else {
                self.0.borrow_mut().insert(key.into(), value.into());
            }
        }
    }

    fn with_secrets() -> SessionState {
        let mut state = populated();
        let draft = state.workspaces.get_mut("/ws").unwrap().draft.as_mut();
        let draft = draft.unwrap();
        draft.basic_password = "pw-secret".into();
        draft.api_key_value = "key-secret".into();
        state
    }

    #[test]
    fn serialized_session_contains_no_secret_values() {
        let json = serde_json::to_string(&with_secrets()).unwrap();
        for needle in [
            "bearer_token",
            "basic_password",
            "api_key_value",
            "\"tok\"",
            "pw-secret",
            "key-secret",
        ] {
            assert!(!json.contains(needle), "{needle} leaked into {json}");
        }
    }

    /// The sync group secret lives only in the keychain, so a restart
    /// rejoins the same group; an unreadable entry starts a new one.
    #[test]
    fn the_group_secret_round_trips_through_the_store() {
        let mock_store = MockStore::default();
        assert_eq!(read_group_secret(&mock_store), None);
        let secret = GroupSecret([5; 32]);
        write_group_secret(&mock_store, &secret);
        assert_eq!(read_group_secret(&mock_store), Some(secret));
        mock_store.set(GROUP_SECRET_KEY, "not base64 of 32 bytes");
        assert_eq!(read_group_secret(&mock_store), None);
    }

    #[test]
    fn secrets_round_trip_through_the_store_keyed_by_workspace() {
        let mock_store = MockStore::default();
        stash(&mut with_secrets(), &mock_store);
        assert_eq!(
            mock_store.get("/ws#basic_password").as_deref(),
            Some("pw-secret")
        );

        let tmp = TempDir::new("session-secrets");
        let file = tmp.join("session.json");
        save_to(&file, &with_secrets());
        let mut back = load_from(&file);
        hydrate(&mut back, &mock_store);
        let draft = back.workspaces["/ws"].draft.as_ref().unwrap();
        assert_eq!(draft.bearer_token, "tok");
        assert_eq!(draft.basic_password, "pw-secret");
        assert_eq!(draft.api_key_value, "key-secret");
    }

    #[test]
    fn an_old_session_with_inline_secrets_loads_and_migrates() {
        let tmp = TempDir::new("session-old-secrets");
        let file = tmp.write(
            "session.json",
            r#"{"workspaces":{"/ws":{"draft":{"bearer_token":"tok-secret","basic_password":"pw-secret"}}}}"#,
        );
        let mock_store = MockStore::default();
        let mut back = load_from(&file);
        hydrate(&mut back, &mock_store);
        let draft = back.workspaces["/ws"].draft.as_ref().unwrap();
        assert_eq!(draft.bearer_token, "tok-secret", "inline value restores");

        stash(&mut back, &mock_store);
        assert_eq!(
            mock_store.get("/ws#bearer_token").as_deref(),
            Some("tok-secret")
        );
        save_to(&file, &back);
        let text = std::fs::read_to_string(&file).unwrap();
        assert!(!text.contains("-secret"), "{text}");
    }

    #[test]
    fn an_empty_store_leaves_the_rest_of_the_session_intact() {
        let mock_store = MockStore::default();
        let mut state = populated();
        state
            .workspaces
            .get_mut("/ws")
            .unwrap()
            .draft
            .as_mut()
            .unwrap()
            .bearer_token = String::new();
        hydrate(&mut state, &mock_store);
        let draft = state.workspaces["/ws"].draft.as_ref().unwrap();
        assert_eq!(draft.bearer_token, "");
        assert_eq!(draft.url, "https://api.example.com/日本語?q=🎉");
    }

    fn populated() -> SessionState {
        let draft = RequestDraft {
            protocol: "graphql".into(),
            active_tab: 3,
            url: "https://api.example.com/日本語?q=🎉".into(),
            method: "POST".into(),
            headers: vec![
                HeaderEntry {
                    key: "X-Trace".into(),
                    value: "e\u{0301}tag".into(),
                    enabled: true,
                },
                HeaderEntry {
                    key: "X-Off".into(),
                    value: String::new(),
                    enabled: false,
                },
            ],
            body: "{\"a\":1}".into(),
            body_type: "json".into(),
            auth_type: "bearer".into(),
            bearer_token: "tok".into(),
            graphql_query: "query { me }".into(),
            grpc_proto_path: Some(PathBuf::from("/protos/a.proto")),
            grpc_service: Some("Svc".into()),
            grpc_method_name: Some("Call".into()),
            ..Default::default()
        };
        let mut state = SessionState {
            current_workspace: Some(PathBuf::from("/ws")),
            workspaces: HashMap::new(),
        };
        state.workspaces.insert(
            "/ws".into(),
            WorkspaceEntry {
                active_file: Some(PathBuf::from("/ws/a.http")),
                active_request: 2,
                draft: Some(draft),
                expanded_folders: vec![PathBuf::from("/ws"), PathBuf::from("/ws/sub")],
                active_env: Some("dev".into()),
            },
        );
        state
    }

    #[test]
    fn round_trips_every_field_through_disk() {
        let tmp = TempDir::new("session");
        let file = tmp.join("session.json");
        save_to(&file, &populated());
        let back = load_from(&file);

        assert_eq!(back.current_workspace, Some(PathBuf::from("/ws")));
        let entry = back.workspaces.get("/ws").expect("workspace restored");
        assert_eq!(entry.active_file, Some(PathBuf::from("/ws/a.http")));
        assert_eq!(entry.active_request, 2);
        assert_eq!(entry.active_env.as_deref(), Some("dev"));
        assert_eq!(entry.expanded_folders.len(), 2);

        let draft = entry.draft.as_ref().expect("draft restored");
        assert_eq!(draft.protocol, "graphql");
        assert_eq!(draft.active_tab, 3);
        assert_eq!(draft.url, "https://api.example.com/日本語?q=🎉");
        assert_eq!(draft.method, "POST");
        assert_eq!(draft.body, "{\"a\":1}");
        assert_eq!(draft.auth_type, "bearer");
        assert_eq!(draft.bearer_token, "", "secrets never reach the file");
        assert_eq!(draft.graphql_query, "query { me }");
        assert_eq!(
            draft.grpc_proto_path,
            Some(PathBuf::from("/protos/a.proto"))
        );
        assert_eq!(draft.grpc_service.as_deref(), Some("Svc"));
        assert_eq!(draft.headers.len(), 2);
        assert_eq!(draft.headers[0].value, "e\u{0301}tag");
        assert!(draft.headers[0].enabled);
        assert!(!draft.headers[1].enabled, "disabled rows stay disabled");
    }

    #[test]
    fn creates_missing_parent_directories() {
        let tmp = TempDir::new("session-mkdir");
        let file = tmp.join("a").join("b").join("session.json");
        save_to(&file, &SessionState::default());
        assert!(file.exists(), "save_to must create intermediate dirs");
    }

    #[test]
    fn missing_file_loads_the_default_session() {
        let tmp = TempDir::new("session-missing");
        let back = load_from(&tmp.join("nope.json"));
        assert!(back.current_workspace.is_none());
        assert!(back.workspaces.is_empty());
    }

    #[test]
    fn a_directory_in_place_of_the_file_loads_the_default_session() {
        let tmp = TempDir::new("session-isdir");
        assert!(load_from(tmp.path()).workspaces.is_empty());
    }

    #[test]
    fn corrupt_files_load_the_default_session_without_panicking() {
        let tmp = TempDir::new("session-corrupt");
        for (name, body) in [
            ("garbage.json", "not json"),
            ("empty.json", ""),
            ("whitespace.json", "  \n "),
            // Killed mid-write: JSON ends abruptly.
            (
                "truncated.json",
                "{\"current_workspace\":\"/ws\",\"workspaces\":{\"/ws\":{\"acti",
            ),
            ("array.json", "[]"),
            ("null.json", "null"),
            ("number.json", "12"),
            // Right shape, wrong types all the way down.
            (
                "wrong-types.json",
                "{\"current_workspace\":42,\"workspaces\":\"nope\"}",
            ),
            ("workspaces-array.json", "{\"workspaces\":[1,2]}"),
            ("nul-bytes.json", "\0\0\0"),
        ] {
            let file = tmp.write(name, body);
            let back = load_from(&file);
            assert!(
                back.current_workspace.is_none() && back.workspaces.is_empty(),
                "{name} should load as the default session"
            );
        }
    }

    #[test]
    fn a_file_that_is_not_valid_utf8_loads_the_default_session() {
        let tmp = TempDir::new("session-binary");
        let file = tmp.join("session.json");
        std::fs::write(&file, [0xffu8, 0xfe, 0x00, 0x80, 0x9f]).unwrap();
        assert!(load_from(&file).workspaces.is_empty());
    }

    // FIXED: none of these structs used to carry `#[serde(default)]`, so a
    // session file written by any build with a different field set - an older
    // Protide, or a newer one after a field is added - failed to deserialize
    // outright. `load()`'s `unwrap_or_default()` then silently discarded the
    // user's *entire* session (workspace, open file, unsaved draft) because of
    // one absent field. These tests pin the tolerant behaviour.
    #[test]
    fn a_session_missing_newer_fields_still_restores_what_it_has() {
        let tmp = TempDir::new("session-old-schema");
        // As written by a build that predates `active_env` and most draft fields.
        let file = tmp.write(
            "session.json",
            r#"{"current_workspace":"/ws","workspaces":{"/ws":{
                 "active_file":"/ws/a.http",
                 "draft":{"url":"https://x.test","method":"GET"},
                 "expanded_folders":["/ws"]}}}"#,
        );
        let back = load_from(&file);
        assert_eq!(back.current_workspace, Some(PathBuf::from("/ws")));
        let entry = back.workspaces.get("/ws").expect("workspace survived");
        assert_eq!(entry.active_file, Some(PathBuf::from("/ws/a.http")));
        assert_eq!(entry.active_env, None, "absent field falls back to default");
        let draft = entry.draft.as_ref().expect("draft survived");
        assert_eq!(draft.url, "https://x.test");
        assert_eq!(draft.method, "GET");
        assert_eq!(draft.protocol, "", "absent field falls back to default");
        assert!(draft.headers.is_empty());
    }

    #[test]
    fn a_top_level_session_missing_the_workspaces_map_still_restores() {
        let tmp = TempDir::new("session-no-map");
        let file = tmp.write("session.json", r#"{"current_workspace":"/ws"}"#);
        assert_eq!(
            load_from(&file).current_workspace,
            Some(PathBuf::from("/ws")),
            "the current workspace must survive an absent workspaces map"
        );
    }

    #[test]
    fn an_empty_json_object_is_a_valid_empty_session() {
        let tmp = TempDir::new("session-empty-obj");
        let file = tmp.write("session.json", "{}");
        let back = load_from(&file);
        assert!(back.current_workspace.is_none());
        assert!(back.workspaces.is_empty());
    }

    #[test]
    fn fields_from_a_newer_build_are_ignored_rather_than_fatal() {
        let tmp = TempDir::new("session-new-schema");
        let file = tmp.write(
            "session.json",
            r#"{"current_workspace":"/ws","workspaces":{},"future_field":{"a":1}}"#,
        );
        assert_eq!(
            load_from(&file).current_workspace,
            Some(PathBuf::from("/ws"))
        );
    }

    #[test]
    fn saving_replaces_rather_than_appends() {
        let tmp = TempDir::new("session-replace");
        let file = tmp.join("session.json");
        save_to(&file, &populated());
        save_to(&file, &SessionState::default());
        let back = load_from(&file);
        assert!(
            back.workspaces.is_empty(),
            "a shorter session must not leave the previous payload's tail behind"
        );
    }

    #[test]
    fn workspace_keys_with_unicode_and_separators_round_trip() {
        let tmp = TempDir::new("session-unicode-key");
        let file = tmp.join("session.json");
        let key = "/Users/ünïcode/プロジェクト/my repo";
        let mut state = SessionState::default();
        state
            .workspaces
            .insert(key.into(), WorkspaceEntry::default());
        save_to(&file, &state);
        assert!(load_from(&file).workspaces.contains_key(key));
    }
}
