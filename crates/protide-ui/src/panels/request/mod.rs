//! Request editor panel – URL input, method selector, headers/params/body, auth.

mod render;
mod render_trpc_playground;
mod render_trpc_pg_sidebar;
mod render_trpc_pg_io;
mod render_url_bar;
mod render_dropdowns;
mod render_tab_router;
mod render_http;
mod render_body_form;
mod render_body;
mod render_kv;
mod render_auth;
mod render_auth_content;
mod render_auth_basic_apikey;
mod render_scripts;
mod render_graphql_query;
mod render_graphql_schema;
mod render_websocket;
mod render_grpc;
mod render_grpc_proto;
mod render_import;
mod render_socketio;
mod render_socketio_helpers;
mod render_data;
mod render_settings;

mod init;
mod state;
mod body;
mod draft_save;
mod draft_load;
mod url_sync;
mod editing_kv;
mod editing_text;
mod editing_keys;
mod editing_url;
mod save;
mod codegen;
mod import;
mod graphql;
mod execution_auth;
mod execution_http;
mod execution_ws;
mod execution_sio;
mod execution_grpc;
mod execution_trpc;
mod panel_state;

#[cfg(test)]
mod tests;

use std::ops::Range;
use std::marker::PhantomData;
use gpui::{
    prelude::*, Context, Entity, FocusHandle, ScrollHandle, Subscription, Window,
};
use gpui_component::input::InputState;
use protide_core::execution::{ExecutionBody, ExecutionMode, ExecutionRequest, FormPart, FormPartValue};
use protide_core::execution::ws::{
    TungsteniteExecutor, WebSocketExecutor, WsCommand, WsConnectionParams, WsDirection, WsEvent,
    WsMessage, WsRingBuffer,
};
use protide_core::execution::sio::{
    SocketIoExecutor, TungsteniteSocketIoExecutor, SioCommand, SioConnectionParams, SioUiEvent, SioRingBuffer,
};
use super::console::{ConsoleEntry, ConsoleEntrySource, ConsolePanel, LogLevel};
use super::explorer::ExplorerPanel;
use super::request_types::{
    ApiKeyLocation, AuthType, BodyType, EditTarget, FormField, FormFieldType,
    GqlSchemaType, GraphqlSchemaState, GrpcMethodInfo, GrpcStreamingType,
    HttpMethod, KeyValuePair, KvList, PendingEditor, RequestMode,
    SioConnectionState, TrpcPlaygroundProc, TrpcProcKind, WsConnectionState,
};
use panel_state::{GrpcPanel, WsPanel, SioPanel, TrpcPanel, GraphqlPanel, ScriptPanel};
use super::response::{ResponseData, ResponsePanel};
use protide_core::codegen::Language as CodegenLanguage;
use http_parser::VariableExtraction;
use crate::last_paths;

fn char_to_byte_offset(text: &str, char_idx: usize) -> usize {
    text.char_indices().nth(char_idx).map(|(b, _)| b).unwrap_or(text.len())
}

/// Request editor panel.
/// `E` is the WebSocket backend (default: `TungsteniteExecutor`).
pub struct RequestPanel<E: WebSocketExecutor = TungsteniteExecutor> {
    pub(super) active_tab: usize,
    pub(super) method: HttpMethod,
    pub(super) url: String,
    pub(super) url_selection: Range<usize>,
    pub(super) method_dropdown_open: bool,
    pub(super) mode_dropdown_open: bool,
    pub(super) url_focus: FocusHandle,
    pub(super) is_selecting: bool,
    pub(super) url_input_left: f32,
    pub(super) url_input_width: f32,
    pub(super) url_scroll_offset: f32,
    _edit_blur_sub: Option<Subscription>,
    pub(super) response_panel: Entity<ResponsePanel>,
    pub(super) loading: bool,
    pub(super) headers: Vec<KeyValuePair>,
    pub(super) params: Vec<KeyValuePair>,
    pub(super) form_data: Vec<FormField>,
    pub(super) body: String,
    pub(super) body_type: BodyType,
    pub(super) binary_file_path: Option<std::path::PathBuf>,
    pub(super) syncing_params: bool,
    pub(super) auth_type: AuthType,
    pub(super) bearer_token: String,
    pub(super) basic_username: String,
    pub(super) basic_password: String,
    pub(super) api_key_name: String,
    pub(super) api_key_value: String,
    pub(super) api_key_location: ApiKeyLocation,
    pub(super) active_edit: Option<EditTarget>,
    pub(super) edit_selection: Range<usize>,
    pub(super) edit_is_selecting: bool,
    pub(super) edit_input_origins: std::collections::HashMap<EditTarget, f32>,
    pub(super) url_undo_stack: std::collections::VecDeque<(String, Range<usize>)>,
    pub(super) url_redo_stack: std::collections::VecDeque<(String, Range<usize>)>,
    pub(super) edit_undo_stack: std::collections::VecDeque<(EditTarget, String, Range<usize>)>,
    pub(super) edit_redo_stack: std::collections::VecDeque<(EditTarget, String, Range<usize>)>,
    pub(super) skip_blur: bool,
    pub(super) edit_focus: FocusHandle,
    pub(super) body_focus: FocusHandle,
    pub(super) explorer_panel: Option<Entity<ExplorerPanel>>,
    pub(super) body_editor: Entity<InputState>,
    pub(super) variable_extractions: Vec<VariableExtraction>,
    pub codegen_content: Option<String>,
    pub codegen_language: CodegenLanguage,
    pub codegen_editor: Entity<InputState>,
    pub import_modal_open: bool,
    pub(super) import_text: String,
    pub(super) import_error: Option<String>,
    pub(super) import_editor: Entity<InputState>,
    pub(super) request_mode: RequestMode,
    pub(super) grpc: GrpcPanel,
    pub(super) ws: WsPanel,
    pub(super) sio: SioPanel,
    pub(super) trpc: TrpcPanel,
    pub(super) graphql: GraphqlPanel,
    pub(super) scripts: ScriptPanel,
    pub(super) kv_col_key_w: f32,
    pub(super) kv_col_drag: Option<(f32, f32)>,
    pub(super) current_file: Option<std::path::PathBuf>,
    pub(super) save_feedback: bool,
    pub(super) custom_method_input: String,
    pub(super) custom_method_focus: FocusHandle,
    pub(super) console_panel: Option<Entity<ConsolePanel>>,
    pub(super) csv_path: Option<std::path::PathBuf>,
    pub(super) data_results: Vec<crate::panels::request_types::DataRunRow>,
    pub(super) data_running: bool,
    pub(super) timeout_input: Entity<gpui_component::input::InputState>,
    pub(super) verify_ssl: bool,
    pub(super) impersonate_browser: bool,
    pub(super) kv_row_drag: Option<(KvList, usize, f32)>,
    pub(super) kv_row_drag_over: Option<usize>,
    pub(super) form_row_drag: Option<(usize, f32)>,
    pub(super) form_row_drag_over: Option<usize>,
    /// Editor content updates deferred until render() (set_value needs &mut Window).
    /// Each entry is (target editor, new content); applied and cleared on next render.
    pub(super) editor_pending: Vec<(PendingEditor, String)>,
    _executor: PhantomData<E>,
}
