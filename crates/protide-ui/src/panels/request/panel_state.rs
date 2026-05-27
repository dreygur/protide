//! Protocol-specific state sub-structs for RequestPanel.
//! Each struct owns only the fields that belong to that protocol.

use gpui::{Entity, ScrollHandle};
use gpui_component::input::InputState;
use protide_core::execution::ws::{WsCommand, WsRingBuffer};
use protide_core::execution::sio::{SioCommand, SioRingBuffer};
use crate::panels::request_types::{
    GrpcMethodInfo, GraphqlSchemaState, KeyValuePair,
    SioConnectionState, TrpcPlaygroundProc, TrpcProcKind, WsConnectionState,
};

pub struct GrpcPanel {
    pub(super) message_editor: Entity<InputState>,
    pub(super) metadata: Vec<KeyValuePair>,
    pub(super) proto_path: Option<std::path::PathBuf>,
    pub(super) proto_content: String,
    pub(super) services: Vec<String>,
    pub(super) service: Option<String>,
    pub(super) methods: Vec<GrpcMethodInfo>,
    pub(super) method: Option<GrpcMethodInfo>,
}

pub struct WsPanel {
    pub(super) state: WsConnectionState,
    pub(super) messages: WsRingBuffer,
    pub(super) message_editor: Entity<InputState>,
    pub(super) send_tx: Option<std::sync::mpsc::Sender<WsCommand>>,
    pub(super) compose_h: f32,
    pub(super) compose_drag: Option<(f32, f32)>,
    pub(super) scroll: ScrollHandle,
}

pub struct SioPanel {
    pub(super) state: SioConnectionState,
    pub(super) messages: SioRingBuffer,
    pub(super) namespace: String,
    pub(super) event_name: String,
    pub(super) want_ack: bool,
    pub(super) next_ack_id: u32,
    pub(super) payload_editor: Entity<InputState>,
    pub(super) send_tx: Option<std::sync::mpsc::Sender<SioCommand>>,
    pub(super) room_name: String,
    pub(super) active_rooms: Vec<String>,
}

pub struct TrpcPanel {
    pub(super) procedure: String,
    pub(super) params_editor: Entity<InputState>,
    pub(super) pg_procedures: Vec<TrpcPlaygroundProc>,
    pub(super) pg_selected: Option<usize>,
    pub(super) pg_loading: bool,
    pub(super) pg_response: Option<String>,
    pub(super) pg_error: Option<String>,
    pub(super) pg_status: Option<u16>,
    pub(super) pg_elapsed: Option<std::time::Duration>,
    pub(super) pg_search_input: Entity<InputState>,
    pub(super) pg_result_viewer: Entity<InputState>,
    pub(super) pg_add_input: Entity<InputState>,
    pub(super) pg_add_kind: TrpcProcKind,
    pub(super) pg_sidebar_w: f32,
    pub(super) pg_sidebar_drag: Option<(f32, f32)>,
    pub(super) pg_schema_loading: bool,
    pub(super) pg_schema_error: Option<String>,
    pub(super) pg_import_url_input: Entity<InputState>,
    pub(super) pg_show_import_url: bool,
    pub(super) pg_editing: Option<usize>,
    pub(super) pg_edit_input: Entity<InputState>,
    pub(super) pg_edit_kind: TrpcProcKind,
    pub(super) pg_editing_group: Option<String>,
    pub(super) pg_group_edit_input: Entity<InputState>,
}

pub struct GraphqlPanel {
    pub(super) query_editor: Entity<InputState>,
    pub(super) variables_editor: Entity<InputState>,
    pub(super) operation_name: String,
    pub(super) schema: GraphqlSchemaState,
    pub(super) schema_search: String,
}

pub struct ScriptPanel {
    pub(super) pre: String,
    pub(super) post: String,
    pub(super) tests: String,
    pub(super) pre_editor: Entity<InputState>,
    pub(super) post_editor: Entity<InputState>,
    pub(super) tests_editor: Entity<InputState>,
    pub(super) pre_open: bool,
    pub(super) post_open: bool,
    pub(super) tests_open: bool,
    pub(super) pre_h: f32,
    pub(super) post_h: f32,
    pub(super) drag_pre: Option<(f32, f32)>,
    pub(super) drag_post: Option<(f32, f32)>,
}
