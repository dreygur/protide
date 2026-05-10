//! Request editor panel
//!
//! The main request builder UI with URL input, method selector,
//! headers/params/body editors, and authentication configuration.

mod render;

#[cfg(test)]
mod tests;

use std::ops::Range;

use log::{error, info};
use gpui::{
    deferred, div, prelude::*, px, ClipboardItem, Context, Entity, FocusHandle, IntoElement,
    KeyDownEvent, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, Render,
    Styled, Subscription, Window,
};

use crate::ui::components::{render_text_view_with_max, find_word_start, find_word_end};
use crate::ui::components::code_editor::{CodeEditor, Language};
use protide_core::execution::{ExecutionBody, ExecutionMode, ExecutionRequest, FormPart, FormPartValue};
use std::marker::PhantomData;

use protide_core::execution::ws::{
    TungsteniteExecutor, WebSocketExecutor, WsCommand, WsConnectionParams, WsDirection, WsEvent,
    WsMessage, WsRingBuffer,
};
use protide_core::execution::sio::{
    SocketIoExecutor, TungsteniteSocketIoExecutor, SioCommand, SioConnectionParams, SioUiEvent, SioRingBuffer,
};

use super::console::{ConsoleEntry, ConsoleEntrySource, ConsolePanel, LogLevel};
use super::explorer::ExplorerPanel;
use super::request_types::{ApiKeyLocation, AuthType, BodyType, EditTarget, FormField, FormFieldType, GrpcMethodInfo, GrpcStreamingType, HttpMethod, KeyValuePair, RequestMode, SioConnectionState, WsConnectionState};
use super::request_utils::{base64_encode, status_text, url_decode, url_encode};
use base64::Engine;
use super::response::{ResponseData, ResponsePanel};

use protide_core::codegen::{self, CodegenRequest, Language as CodegenLanguage};
use protide_core::import;
use http_parser::VariableExtraction;
use crate::last_paths;

/// Summary of a single type from a GraphQL schema introspection response.
#[derive(Clone, Debug)]
pub struct GqlSchemaType {
    pub name: String,
    pub kind: String,
    pub description: Option<String>,
}

/// State of the GraphQL schema for the current endpoint.
#[derive(Clone, Debug, Default)]
pub enum GraphqlSchemaState {
    #[default]
    Idle,
    Loading,
    Loaded(Vec<GqlSchemaType>),
    Error(String),
}

/// Helper to render text with selection highlighting
fn render_text_view(
    text: &str,
    selection: &Range<usize>,
    is_focused: bool,
    font_size: f32,
    text_color: gpui::Hsla,
    placeholder: Option<&str>,
    placeholder_color: gpui::Hsla,
    selection_bg: gpui::Hsla,
) -> gpui::AnyElement {
    render_text_view_with_max(text, selection, is_focused, font_size, text_color, placeholder, placeholder_color, None, selection_bg)
}

/// Convert character index to byte offset in a string
fn char_to_byte_offset(text: &str, char_idx: usize) -> usize {
    text.char_indices()
        .nth(char_idx)
        .map(|(byte_offset, _)| byte_offset)
        .unwrap_or(text.len())
}

/// Convert byte offset to character index in a string
#[allow(dead_code)]
fn byte_to_char_offset(text: &str, byte_offset: usize) -> usize {
    text[..byte_offset.min(text.len())]
        .chars()
        .count()
}

/// Request editor panel.
///
/// `E` is the WebSocket backend. The default is `TungsteniteExecutor` (production).
/// Tests can supply a different type that implements `WebSocketExecutor` to inject
/// mock connections without touching the UI rendering logic.
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
    /// Actual window-x origin per input, captured by canvas() each frame
    pub(super) edit_input_origins: std::collections::HashMap<EditTarget, f32>,
    pub(super) url_undo_stack: Vec<(String, Range<usize>)>,
    pub(super) url_redo_stack: Vec<(String, Range<usize>)>,
    pub(super) edit_undo_stack: Vec<(EditTarget, String, Range<usize>)>,
    pub(super) edit_redo_stack: Vec<(EditTarget, String, Range<usize>)>,
    pub(super) skip_blur: bool,
    pub(super) edit_focus: FocusHandle,
    pub(super) body_focus: FocusHandle,
    pub(super) explorer_panel: Option<Entity<ExplorerPanel>>,
    // CodeEditor for JSON/Raw body
    pub(super) body_editor: Entity<CodeEditor>,
    // Script editors
    pub(super) pre_script: String,
    pub(super) post_script: String,
    pub(super) tests: String,
    pub(super) pre_script_editor: Entity<CodeEditor>,
    pub(super) post_script_editor: Entity<CodeEditor>,
    pub(super) tests_editor: Entity<CodeEditor>,
    /// Variable extractions from @set annotations
    pub(super) variable_extractions: Vec<VariableExtraction>,
    /// Generated code content
    pub codegen_content: Option<String>,
    /// Selected code generation language
    pub codegen_language: CodegenLanguage,
    /// Read-only editor for generated code display
    pub codegen_editor: Entity<CodeEditor>,
    /// Import modal open state
    pub import_modal_open: bool,
    /// Import text input content
    pub(super) import_text: String,
    /// Import error message
    pub(super) import_error: Option<String>,
    /// Code editor for import modal text area
    pub(super) import_editor: Entity<CodeEditor>,
    /// Request mode (HTTP or GraphQL)
    pub(super) request_mode: RequestMode,
    /// GraphQL query editor
    pub(super) graphql_query_editor: Entity<CodeEditor>,
    /// GraphQL variables editor
    pub(super) graphql_variables_editor: Entity<CodeEditor>,
    /// GraphQL operation name (optional)
    pub(super) graphql_operation_name: String,
    /// WebSocket connection state
    pub(super) ws_state: WsConnectionState,
    /// WebSocket message history
    pub(super) ws_messages: WsRingBuffer,
    /// WebSocket message input
    pub(super) ws_message_input: String,
    /// WebSocket message editor
    pub(super) ws_message_editor: Entity<CodeEditor>,
    /// Channel to send messages to WebSocket thread
    ws_send_tx: Option<std::sync::mpsc::Sender<WsCommand>>,
    /// gRPC message editor (JSON/Protobuf)
    pub(super) grpc_message_editor: Entity<CodeEditor>,
    /// gRPC metadata (key-value pairs, similar to headers)
    pub(super) grpc_metadata: Vec<KeyValuePair>,
    /// Proto file path
    pub(super) grpc_proto_path: Option<std::path::PathBuf>,
    /// Proto file content (for display)
    pub(super) grpc_proto_content: String,
    /// Available gRPC services (parsed from proto)
    pub(super) grpc_services: Vec<String>,
    /// Selected gRPC service
    pub(super) grpc_service: Option<String>,
    /// Available methods for selected service with streaming type
    pub(super) grpc_methods: Vec<GrpcMethodInfo>,
    /// Selected gRPC method
    pub(super) grpc_method: Option<GrpcMethodInfo>,

    // tRPC fields
    /// tRPC procedure name (e.g., "query.getUser")
    pub(super) trpc_procedure: String,
    /// tRPC parameters editor
    pub(super) trpc_params_editor: Entity<CodeEditor>,

    // Socket.IO fields
    pub(super) sio_state: SioConnectionState,
    pub(super) sio_messages: SioRingBuffer,
    /// Socket.IO namespace (default "/")
    pub(super) sio_namespace: String,
    /// Event name to emit
    pub(super) sio_event_name: String,
    /// Whether to request an acknowledgement on the next emit
    pub(super) sio_want_ack: bool,
    /// Monotonically increasing ack ID for outgoing events
    pub(super) sio_next_ack_id: u32,
    /// JSON payload editor for the event body
    pub(super) sio_payload_editor: Entity<CodeEditor>,
    sio_send_tx: Option<std::sync::mpsc::Sender<SioCommand>>,
    /// Width of KEY column in KV tables (params, headers, grpc metadata)
    pub(super) kv_col_key_w: f32,
    /// Active KV column drag: (start_x, start_width)
    pub(super) kv_col_drag: Option<(f32, f32)>,
    /// Whether each script section is expanded
    pub(super) script_pre_open: bool,
    pub(super) script_post_open: bool,
    pub(super) script_tests_open: bool,
    /// Script editor heights when expanded
    pub(super) script_pre_h: f32,
    pub(super) script_post_h: f32,
    /// Active script drag: (start_mouse_y, start_height)
    pub(super) drag_script_pre: Option<(f32, f32)>,
    pub(super) drag_script_post: Option<(f32, f32)>,
    /// Current file path (set when a .http file is loaded; enables save-in-place)
    pub(super) current_file: Option<std::path::PathBuf>,
    /// Shows "Saved!" briefly after in-place save
    pub(super) save_feedback: bool,
    /// Draft text for custom HTTP method input
    pub(super) custom_method_input: String,
    pub(super) custom_method_focus: FocusHandle,
    /// GraphQL schema fetched via introspection or imported from file.
    pub(super) graphql_schema: GraphqlSchemaState,
    /// Filter string for the Schema type list.
    pub(super) graphql_schema_search: String,
    /// Optional console panel for logging all outbound requests.
    pub(super) console_panel: Option<Entity<ConsolePanel>>,
    /// Zero-size marker binding the executor type to the panel.
    _executor: PhantomData<E>,
}

impl<E: WebSocketExecutor> RequestPanel<E> {
    pub fn new(cx: &mut Context<Self>, response_panel: Entity<ResponsePanel>) -> Self {
        let url = "https://httpbin.org/post".to_string();
        let url_len = url.len();
        let initial_body = "{\n  \"name\": \"Protide\",\n  \"version\": \"0.1.0\"\n}";
        let body_editor = cx.new(|cx| {
            CodeEditor::new(cx)
                .with_content(initial_body)
                .with_language(Language::Json)
                .with_line_numbers(true)
        });
        let pre_script_editor = cx.new(|cx| {
            CodeEditor::new(cx)
                .with_language(Language::JavaScript)
                .with_line_numbers(true)
        });
        let post_script_editor = cx.new(|cx| {
            CodeEditor::new(cx)
                .with_language(Language::JavaScript)
                .with_line_numbers(true)
        });
        let tests_editor = cx.new(|cx| {
            CodeEditor::new(cx)
                .with_language(Language::JavaScript)
                .with_line_numbers(true)
        });
        let graphql_query_editor = cx.new(|cx| {
            CodeEditor::new(cx)
                .with_content("query {\n  \n}")
                .with_language(Language::GraphQL)
                .with_line_numbers(true)
        });
        let graphql_variables_editor = cx.new(|cx| {
            CodeEditor::new(cx)
                .with_content("{}")
                .with_language(Language::Json)
                .with_line_numbers(true)
        });
        let ws_message_editor = cx.new(|cx| {
            CodeEditor::new(cx)
                .with_content("{\"type\": \"hello\"}")
                .with_language(Language::Json)
                .with_line_numbers(true)
        });
        let grpc_message_editor = cx.new(|cx| {
            CodeEditor::new(cx)
                .with_content("{}")
                .with_language(Language::Json)
                .with_line_numbers(true)
        });
        let trpc_params_editor = cx.new(|cx| {
            CodeEditor::new(cx)
                .with_content("{}")
                .with_language(Language::Json)
                .with_line_numbers(true)
        });
        let sio_payload_editor = cx.new(|cx| {
            CodeEditor::new(cx)
                .with_content("{}")
                .with_language(Language::Json)
        });
        let codegen_editor = cx.new(|cx| {
            CodeEditor::new(cx)
                .with_read_only(true)
                .with_line_numbers(true)
        });
        let import_editor = cx.new(|cx| {
            CodeEditor::new(cx)
                .with_language(Language::Plain)
                .with_line_numbers(false)
        });
        Self {
            active_tab: 0,
            method: HttpMethod::Post,
            url,
            url_selection: url_len..url_len,
            method_dropdown_open: false,
            mode_dropdown_open: false,
            url_focus: cx.focus_handle(),
            is_selecting: false,
            url_input_left: 0.0,
            url_input_width: 400.0,
            url_scroll_offset: 0.0,
            _edit_blur_sub: None,
            response_panel,
            loading: false,
            headers: vec![
                KeyValuePair {
                    key: "Content-Type".to_string(),
                    value: "application/json".to_string(),
                    enabled: true,
                },
                KeyValuePair::default(),
                KeyValuePair::default(),
            ],
            params: vec![
                KeyValuePair::default(),
                KeyValuePair::default(),
                KeyValuePair::default(),
            ],
            form_data: vec![FormField::default()],
            body: initial_body.to_string(),
            body_type: BodyType::Json,
            binary_file_path: None,
            syncing_params: false,
            auth_type: AuthType::None,
            bearer_token: String::new(),
            basic_username: String::new(),
            basic_password: String::new(),
            api_key_name: String::new(),
            api_key_value: String::new(),
            api_key_location: ApiKeyLocation::Header,
            active_edit: None,
            edit_selection: 0..0,
            edit_is_selecting: false,
            edit_input_origins: std::collections::HashMap::new(),
            url_undo_stack: Vec::new(),
            url_redo_stack: Vec::new(),
            edit_undo_stack: Vec::new(),
            edit_redo_stack: Vec::new(),
            skip_blur: false,
            edit_focus: cx.focus_handle(),
            body_focus: cx.focus_handle(),
            explorer_panel: None,
            body_editor,
            pre_script: String::new(),
            post_script: String::new(),
            tests: String::new(),
            pre_script_editor,
            post_script_editor,
            tests_editor,
            variable_extractions: Vec::new(),
            codegen_content: None,
            codegen_language: CodegenLanguage::Curl,
            codegen_editor,
            import_modal_open: false,
            import_text: String::new(),
            import_error: None,
            import_editor,
            request_mode: RequestMode::Http,
            graphql_query_editor,
            graphql_variables_editor,
            graphql_operation_name: String::new(),
            ws_state: WsConnectionState::Disconnected,
            ws_messages: WsRingBuffer::default(),
            ws_message_input: String::new(),
            ws_message_editor,
            ws_send_tx: None,
            grpc_message_editor,
            grpc_metadata: vec![KeyValuePair::default()],
            grpc_proto_path: None,
            grpc_proto_content: String::new(),
            grpc_services: Vec::new(),
            grpc_service: None,
            grpc_methods: Vec::new(),
            grpc_method: None,
            trpc_procedure: String::new(),
            trpc_params_editor,
            sio_state: SioConnectionState::Disconnected,
            sio_messages: SioRingBuffer::default(),
            sio_namespace: "/".to_string(),
            sio_event_name: "message".to_string(),
            sio_want_ack: false,
            sio_next_ack_id: 1,
            sio_payload_editor,
            sio_send_tx: None,
            kv_col_key_w: 150.0,
            kv_col_drag: None,
            script_pre_open: true,
            script_post_open: true,
            script_tests_open: true,
            script_pre_h: crate::prefs::get_f32("request.script_pre_h", 160.0),
            script_post_h: crate::prefs::get_f32("request.script_post_h", 160.0),
            drag_script_pre: None,
            drag_script_post: None,
            current_file: None,
            save_feedback: false,
            custom_method_input: String::new(),
            custom_method_focus: cx.focus_handle(),
            graphql_schema: GraphqlSchemaState::Idle,
            graphql_schema_search: String::new(),
            console_panel: None,
            _executor: PhantomData,
        }
    }

    /// Set the explorer panel reference for environment variable substitution
    pub fn set_explorer_panel(&mut self, explorer_panel: Entity<ExplorerPanel>, cx: &mut Context<Self>) {
        self.explorer_panel = Some(explorer_panel);
        cx.notify();
    }

    /// Connect the shared console panel so every request is logged.
    pub fn set_console_panel(&mut self, console: Entity<ConsolePanel>, cx: &mut Context<Self>) {
        self.console_panel = Some(console);
        cx.notify();
    }

    pub fn has_response_panel(&self) -> bool {
        !matches!(self.request_mode, RequestMode::WebSocket | RequestMode::SocketIo)
    }

    /// Get the current request mode label for status bar display
    pub fn mode_label(&self) -> &'static str {
        match self.request_mode {
            RequestMode::Http => "HTTP",
            RequestMode::GraphQL => "GraphQL",
            RequestMode::WebSocket => "WebSocket",
            RequestMode::Grpc => "gRPC",
            RequestMode::Trpc => "tRPC",
            RequestMode::SocketIo => "Socket.IO",
        }
    }

    /// Set request mode (HTTP, GraphQL, or WebSocket)
    pub(super) fn set_request_mode(&mut self, mode: RequestMode, cx: &mut Context<Self>) {
        if self.request_mode == mode {
            return;
        }
        self.request_mode = mode;
        self.active_tab = 0; // Reset to first tab
        match mode {
            RequestMode::GraphQL => {
                self.method = HttpMethod::Post;
            }
            RequestMode::WebSocket => {
                // WebSocket uses ws:// or wss:// URL
                if !self.url.starts_with("ws://") && !self.url.starts_with("wss://") {
                    self.url = "wss://echo.websocket.org".to_string();
                    let len = self.url.chars().count();
                    self.url_selection = len..len;
                }
            }
            RequestMode::Grpc => {
                // gRPC uses grpc:// URL scheme
                if !self.url.contains("grpc") {
                    self.url = "grpc://localhost:50051".to_string();
                    let len = self.url.chars().count();
                    self.url_selection = len..len;
                }
            }
            RequestMode::Trpc => {
                self.method = HttpMethod::Post;
                if !self.url.ends_with("/trpc") {
                    self.url = "http://localhost:3000/trpc".to_string();
                    let len = self.url.chars().count();
                    self.url_selection = len..len;
                }
            }
            RequestMode::SocketIo => {
                if !self.url.starts_with("http://") && !self.url.starts_with("https://") {
                    self.url = "http://localhost:3000".to_string();
                    let len = self.url.chars().count();
                    self.url_selection = len..len;
                }
            }
            RequestMode::Http => {}
        }
        cx.notify();
    }

    /// Connect to WebSocket server
    pub(super) fn connect_websocket(&mut self, cx: &mut Context<Self>) {
        if !matches!(self.ws_state, WsConnectionState::Disconnected | WsConnectionState::Error) {
            return;
        }

        self.ws_state = WsConnectionState::Connecting;
        self.ws_messages.clear();
        cx.notify();

        let env_state = self.explorer_panel.as_ref().map(|p| p.read(cx).env_state().clone());
        let substitute = |s: &str| -> String {
            env_state.as_ref().map_or_else(|| s.to_string(), |e| e.substitute(s))
        };

        let url = substitute(&self.url);
        let headers: Vec<(String, String)> = self
            .headers
            .iter()
            .filter(|h| h.enabled && !h.key.is_empty())
            .map(|h| (substitute(&h.key), substitute(&h.value)))
            .collect();
        let on_message_script = self.pre_script_editor.read(cx).content().to_string();
        let env_vars: std::collections::HashMap<String, String> = env_state
            .as_ref()
            .and_then(|e| e.active())
            .map(|env| env.variables.clone())
            .unwrap_or_default();
        let explorer_panel = self.explorer_panel.clone();
        let ws_console_panel = self.console_panel.clone();
        let ws_log_url = url.clone();
        info!("WS connecting: {}", ws_log_url);
        let ws_protocol = match self.request_mode {
            RequestMode::SocketIo => "Socket.IO",
            _ => "WebSocket",
        }.to_string();

        let handle = E::connect(WsConnectionParams {
            url,
            headers,
            on_message_script,
            env_vars,
        });
        self.ws_send_tx = Some(handle.cmd_tx);

        let event_rx = handle.event_rx;
        let (fwd_tx, fwd_rx) = async_channel::unbounded::<WsEvent>();
        std::thread::spawn(move || {
            while let Ok(ev) = event_rx.recv() {
                if fwd_tx.try_send(ev).is_err() { break; }
            }
        });
        cx.spawn(async move |this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
            while let Ok(event) = fwd_rx.recv().await {
                match event {
                    WsEvent::Connected => {
                        info!("WS connected: {}", ws_log_url);
                        let _ = cx.update(|cx| {
                            let _ = this.update(cx, |this, cx| {
                                this.ws_state = WsConnectionState::Connected;
                                cx.notify();
                            });
                        });
                    }
                    WsEvent::Message { msg, env_changes } => {
                        let _ = cx.update(|cx| {
                            let _ = this.update(cx, |this, cx| {
                                for (k, v) in &env_changes {
                                    if let Some(ref ep) = explorer_panel {
                                        ep.update(cx, |p, cx| p.set_env_variable(k, v, cx));
                                    }
                                }
                                this.ws_messages.push(msg);
                                cx.notify();
                            });
                        });
                    }
                    WsEvent::Disconnected => {
                        info!("WS disconnected: {}", ws_log_url);
                        let _ = cx.update(|cx| {
                            let _ = this.update(cx, |this, cx| {
                                this.ws_state = WsConnectionState::Disconnected;
                                this.ws_send_tx = None;
                                cx.notify();
                            });
                        });
                        break;
                    }
                    WsEvent::Error(e) => {
                        error!("WS error {}: {}", ws_log_url, e);
                        let _ = cx.update(|cx| {
                            let hint = dns_troubleshoot_hint(&e);
                            if let Some(ref console) = ws_console_panel {
                                let entry = ConsoleEntry {
                                    timestamp: chrono::Local::now(),
                                    level: LogLevel::Error,
                                    source: ConsoleEntrySource::Request,
                                    protocol: ws_protocol.clone(),
                                    method: "CONNECT".to_string(),
                                    url: ws_log_url.clone(),
                                    status: 0,
                                    duration_ms: 0,
                                    error: Some(e.clone()),
                                    response_body: String::new(),
                                    troubleshoot_hint: hint,
                                };
                                console.update(cx, |panel, cx| panel.log(entry, cx));
                            }
                            let _ = this.update(cx, |this, cx| {
                                this.ws_state = WsConnectionState::Error;
                                this.ws_send_tx = None;
                                this.ws_messages.push(WsMessage {
                                    direction: WsDirection::Received,
                                    content: format!("Connection failed: {}", e),
                                    timestamp: chrono::Local::now(),
                                });
                                cx.notify();
                            });
                        });
                        break;
                    }
                }
            }
            let _ = cx.update(|cx| {
                let _ = this.update(cx, |this, cx| {
                    if !matches!(this.ws_state, WsConnectionState::Disconnected | WsConnectionState::Error) {
                        this.ws_state = WsConnectionState::Disconnected;
                        this.ws_send_tx = None;
                        cx.notify();
                    }
                });
            });
        }).detach();
    }

    /// Disconnect from WebSocket server
    pub(super) fn disconnect_websocket(&mut self, cx: &mut Context<Self>) {
        if let Some(tx) = self.ws_send_tx.take() {
            let _ = tx.send(WsCommand::Disconnect);
        }
        self.ws_state = WsConnectionState::Disconnected;
        cx.notify();
    }

    /// Send a message over WebSocket
    pub(super) fn send_websocket_message(&mut self, cx: &mut Context<Self>) {
        if self.ws_state != WsConnectionState::Connected {
            return;
        }

        let message = self.ws_message_editor.read(cx).content();
        if message.trim().is_empty() {
            return;
        }

        if let Some(tx) = &self.ws_send_tx {
            let _ = tx.send(WsCommand::Send(message.to_string()));
            cx.notify();
        }
    }

    /// Connect to a Socket.IO server
    pub(super) fn connect_socketio(&mut self, cx: &mut Context<Self>) {
        if self.sio_state != SioConnectionState::Disconnected {
            return;
        }
        self.sio_state = SioConnectionState::Connecting;
        self.sio_messages.clear();
        cx.notify();

        let env_state = self.explorer_panel.as_ref().map(|p| p.read(cx).env_state().clone());
        let substitute = |s: &str| -> String {
            env_state.as_ref().map_or_else(|| s.to_string(), |e| e.substitute(s))
        };

        let headers: Vec<(String, String)> = self
            .headers
            .iter()
            .filter(|h| h.enabled && !h.key.is_empty())
            .map(|h| (substitute(&h.key), substitute(&h.value)))
            .collect();

        let sio_url = substitute(&self.url);
        info!("SIO connecting: {}", sio_url);
        let handle = TungsteniteSocketIoExecutor::connect(SioConnectionParams {
            url: sio_url,
            namespace: self.sio_namespace.clone(),
            headers,
        });
        self.sio_send_tx = Some(handle.cmd_tx);

        let event_rx = handle.event_rx;
        let (fwd_tx, fwd_rx) = async_channel::unbounded::<SioUiEvent>();
        std::thread::spawn(move || {
            while let Ok(ev) = event_rx.recv() {
                if fwd_tx.try_send(ev).is_err() { break; }
            }
        });
        cx.spawn(async move |this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
            while let Ok(event) = fwd_rx.recv().await {
                match event {
                    SioUiEvent::Connected { .. } => {
                        let _ = cx.update(|cx| {
                            let _ = this.update(cx, |this, cx| {
                                this.sio_state = SioConnectionState::Connected;
                                cx.notify();
                            });
                        });
                    }
                    SioUiEvent::Event(event) => {
                        let _ = cx.update(|cx| {
                            let _ = this.update(cx, |this, cx| {
                                this.sio_messages.push(event);
                                cx.notify();
                            });
                        });
                    }
                    SioUiEvent::Disconnected => {
                        let _ = cx.update(|cx| {
                            let _ = this.update(cx, |this, cx| {
                                this.sio_state = SioConnectionState::Disconnected;
                                this.sio_send_tx = None;
                                cx.notify();
                            });
                        });
                        break;
                    }
                    SioUiEvent::Error(e) => {
                        error!("SIO error: {}", e);
                        let _ = cx.update(|cx| {
                            let _ = this.update(cx, |this, cx| {
                                this.sio_state = SioConnectionState::Disconnected;
                                this.sio_send_tx = None;
                                this.sio_messages.push(protide_core::execution::sio::SioEvent {
                                    direction: protide_core::execution::sio::SioDirection::Received,
                                    namespace: "/".into(),
                                    event_name: "error".into(),
                                    payload: format!("\"{}\"", e),
                                    ack_id: None,
                                    is_ack: false,
                                    timestamp: chrono::Local::now(),
                                });
                                cx.notify();
                            });
                        });
                        break;
                    }
                }
            }
            let _ = cx.update(|cx| {
                let _ = this.update(cx, |this, cx| {
                    if this.sio_state != SioConnectionState::Disconnected {
                        this.sio_state = SioConnectionState::Disconnected;
                        this.sio_send_tx = None;
                        cx.notify();
                    }
                });
            });
        }).detach();
    }

    /// Disconnect from Socket.IO server
    pub(super) fn disconnect_socketio(&mut self, cx: &mut Context<Self>) {
        if let Some(tx) = self.sio_send_tx.take() {
            let _ = tx.send(SioCommand::Disconnect);
        }
        self.sio_state = SioConnectionState::Disconnected;
        cx.notify();
    }

    /// Emit a Socket.IO event
    pub(super) fn emit_socketio_event(&mut self, cx: &mut Context<Self>) {
        if self.sio_state != SioConnectionState::Connected {
            return;
        }
        let payload = self.sio_payload_editor.read(cx).content().to_string();
        let ack_id = if self.sio_want_ack {
            let id = self.sio_next_ack_id;
            self.sio_next_ack_id = self.sio_next_ack_id.wrapping_add(1);
            Some(id)
        } else {
            None
        };
        if let Some(tx) = &self.sio_send_tx {
            let _ = tx.send(SioCommand::Emit {
                namespace: self.sio_namespace.clone(),
                event_name: self.sio_event_name.clone(),
                payload,
                ack_id,
            });
            cx.notify();
        }
    }

    /// Load a proto file for gRPC
    pub(super) fn load_proto_file(&mut self, cx: &mut Context<Self>) {
        use rfd::FileDialog;

        let mut dialog = FileDialog::new()
            .add_filter("Proto Files", &["proto"])
            .set_title("Select Proto File");
        if let Some(dir) = last_paths::last_dir("proto_file") {
            dialog = dialog.set_directory(dir);
        }
        let path = dialog.pick_file();

        if let Some(path) = path {
            last_paths::save_last_dir("proto_file", &path);
            // Read proto file content
            match std::fs::read_to_string(&path) {
                Ok(content) => {
                    self.grpc_proto_path = Some(path);
                    self.grpc_proto_content = content.clone();
                    self.parse_proto_services(&content);
                    info!("Proto loaded: {} ({} services)", self.grpc_proto_path.as_ref().unwrap().display(), self.grpc_services.len());
                    cx.notify();
                }
                Err(e) => {
                    error!("Failed to read proto file: {}", e);
                }
            }
        }
    }

    /// Load a proto file directly from a path (used when restoring from .http file)
    /// Fetch the GraphQL schema via an introspection query to `self.url`.
    pub(super) fn fetch_graphql_schema(&mut self, cx: &mut Context<Self>) {
        let url = if let Some(ref exp) = self.explorer_panel {
            exp.read(cx).env_state().substitute(&self.url)
        } else {
            self.url.clone()
        };
        if url.is_empty() {
            return;
        }

        self.graphql_schema = GraphqlSchemaState::Loading;
        cx.notify();

        cx.spawn(async move |this, mut cx| {
            let result = cx.background_executor()
                .spawn(async move {
                    run_graphql_introspection(&url)
                })
                .await;
            let _ = cx.update(|cx| {
                let _ = this.update(cx, |panel, cx| {
                    panel.graphql_schema = result;
                    cx.notify();
                });
            });
        }).detach();
    }

    /// Import a GraphQL schema from a local .graphql or .json file.
    pub(super) fn import_graphql_schema_file(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, mut cx| {
            let picked = rfd::AsyncFileDialog::new()
                .add_filter("GraphQL Schema", &["graphql", "gql", "json"])
                .pick_file()
                .await;
            if let Some(file) = picked {
                let path = file.path().to_path_buf();
                let result = cx.background_executor()
                    .spawn(async move { parse_schema_file(&path) })
                    .await;
                let _ = cx.update(|cx| {
                    let _ = this.update(cx, |panel, cx| {
                        panel.graphql_schema = result;
                        cx.notify();
                    });
                });
            }
        }).detach();
    }

    pub(super) fn load_grpc_proto_from_path(&mut self, path: std::path::PathBuf, cx: &mut Context<Self>) {
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                self.grpc_proto_path = Some(path);
                self.grpc_proto_content = content.clone();
                self.parse_proto_services(&content);
                info!("Proto loaded: {} ({} services)", self.grpc_proto_path.as_ref().unwrap().display(), self.grpc_services.len());
                cx.notify();
            }
            Err(e) => {
                error!("Failed to read proto file: {}", e);
            }
        }
    }

    /// Parse services and methods from proto file using protox, falling back to text parsing.
    fn parse_proto_services(&mut self, content: &str) {
        self.grpc_services.clear();
        self.grpc_methods.clear();
        self.grpc_service = None;
        self.grpc_method = None;

        // Try real proto parsing via protox first
        if let Some(ref path) = self.grpc_proto_path.clone() {
            if let Ok(pool) = protide_core::protocols::grpc::parse_proto_file(path) {
                for svc in pool.services() {
                    let svc_name = svc.full_name().to_string();
                    self.grpc_services.push(svc_name.clone());
                    for method in svc.methods() {
                        let streaming_type = match (method.is_client_streaming(), method.is_server_streaming()) {
                            (false, false) => GrpcStreamingType::Unary,
                            (false, true) => GrpcStreamingType::ServerStreaming,
                            (true, false) => GrpcStreamingType::ClientStreaming,
                            (true, true) => GrpcStreamingType::BidiStreaming,
                        };
                        self.grpc_methods.push(GrpcMethodInfo {
                            full_name: format!("{}/{}", svc_name, method.name()),
                            streaming_type,
                        });
                    }
                }
                if let Some(s) = self.grpc_services.first() {
                    self.grpc_service = Some(s.clone());
                }
                if let Some(m) = self.grpc_methods.first() {
                    self.grpc_method = Some(m.clone());
                }
                return;
            }
        }

        // Fallback: basic text parsing (streaming unknown, default to unary)
        let mut in_service = false;
        let mut current_service = String::new();

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("service ") {
                if let Some(name) = trimmed.strip_prefix("service ").and_then(|s| s.split_whitespace().next()) {
                    current_service = name.to_string();
                    self.grpc_services.push(current_service.clone());
                    in_service = true;
                }
            }
            if in_service && trimmed.starts_with("rpc ") {
                if let Some(name) = trimmed
                    .strip_prefix("rpc ")
                    .and_then(|s| s.split('(').next())
                    .map(|s| s.trim())
                {
                    self.grpc_methods.push(GrpcMethodInfo {
                        full_name: format!("{}/{}", current_service, name),
                        streaming_type: GrpcStreamingType::Unary,
                    });
                }
            }
            if in_service && trimmed == "}" {
                in_service = false;
            }
        }

        if let Some(s) = self.grpc_services.first() {
            self.grpc_service = Some(s.clone());
        }
        if let Some(m) = self.grpc_methods.first() {
            self.grpc_method = Some(m.clone());
        }
    }

    /// Send a gRPC request
    pub(super) fn send_grpc_request(&mut self, cx: &mut Context<Self>) {
        let Some(method) = &self.grpc_method else {
            return;
        };
        let Some(proto_path) = self.grpc_proto_path.clone() else {
            return;
        };

        self.loading = true;
        cx.notify();

        let message = self.grpc_message_editor.read(cx).content().to_string();
        let url = self.url.clone();
        let method = method.clone();
        let streaming_type = method.streaming_type;

        let env_state = self.explorer_panel.as_ref().map(|p| p.read(cx).env_state().clone());
        let substitute = |s: &str| -> String {
            if let Some(ref env) = env_state { env.substitute(s) } else { s.to_string() }
        };

        let url = substitute(&url);

        let metadata: Vec<(String, String)> = self.grpc_metadata
            .iter()
            .filter(|m| m.enabled && !m.key.is_empty())
            .map(|m| (substitute(&m.key), substitute(&m.value)))
            .collect();

        let response_panel = self.response_panel.clone();

        info!("gRPC {} {}", url, method.full_name);

        match streaming_type {
            GrpcStreamingType::Unary => {
                let task = cx.spawn(async move |this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
                    let (result_tx, result_rx) = std::sync::mpsc::channel::<Result<(String, std::time::Duration), String>>();

                    std::thread::spawn(move || {
                        let result = protide_core::protocols::grpc::execute_unary_blocking(
                            &url,
                            &method.full_name,
                            &message,
                            metadata,
                            &proto_path,
                        );
                        let _ = result_tx.send(result);
                    });

                    if let Ok(result) = result_rx.recv_timeout(std::time::Duration::from_secs(60)) {
                        match result {
                            Ok((body, elapsed)) => {
                                let body_size = body.len();
                                let _ = cx.update(|cx| {
                                    response_panel.update(cx, |panel, cx| {
                                        panel.set_response(ResponseData {
                                            status: 200,
                                            status_text: "OK".to_string(),
                                            headers: vec![
                                                ("content-type".to_string(), "application/grpc+json".to_string()),
                                                ("grpc-status".to_string(), "0".to_string()),
                                            ],
                                            body,
                                            time: elapsed,
                                            size: body_size,
                                        }, cx);
                                    });
                                });
                            }
                            Err(e) => {
                                error!("gRPC error: {}", e);
                                let _ = cx.update(|cx| {
                                    response_panel.update(cx, |panel, cx| {
                                        panel.set_response(ResponseData {
                                            status: 0,
                                            status_text: "Error".to_string(),
                                            headers: vec![],
                                            body: format!("gRPC Error: {}", e),
                                            time: std::time::Duration::ZERO,
                                            size: 0,
                                        }, cx);
                                    });
                                });
                            }
                        }
                        let _ = cx.update(|cx| {
                            let _ = this.update(cx, |panel, cx| {
                                panel.loading = false;
                                cx.notify();
                            });
                        });
                    }
                });
                task.detach();
            }
            GrpcStreamingType::ServerStreaming => {
                let task = cx.spawn(async move |this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
                    let result = protide_core::protocols::grpc::execute_server_streaming(
                        &url,
                        &method.full_name,
                        &message,
                        metadata,
                        &proto_path,
                    ).await;

                    match result {
                        Ok(chunks) => {
                            let body = chunks.join("\n---\n");
                            let body_size = body.len();
                            let _ = cx.update(|cx| {
                                response_panel.update(cx, |panel, cx| {
                                    panel.set_response(ResponseData {
                                        status: 200,
                                        status_text: "OK (streaming)".to_string(),
                                        headers: vec![
                                            ("content-type".to_string(), "application/grpc+json".to_string()),
                                            ("grpc-status".to_string(), "0".to_string()),
                                            ("x-streaming".to_string(), "true".to_string()),
                                        ],
                                        body,
                                        time: std::time::Duration::from_secs(1),
                                        size: body_size,
                                    }, cx);
                                });
                            });
                        }
                        Err(e) => {
                            error!("gRPC streaming error: {}", e);
                            let _ = cx.update(|cx| {
                                response_panel.update(cx, |panel, cx| {
                                    panel.set_response(ResponseData {
                                        status: 0,
                                        status_text: "Error".to_string(),
                                        headers: vec![],
                                        body: format!("gRPC Streaming Error: {}", e),
                                        time: std::time::Duration::ZERO,
                                        size: 0,
                                    }, cx);
                                });
                            });
                        }
                    }
                    let _ = cx.update(|cx| {
                        let _ = this.update(cx, |panel, cx| {
                            panel.loading = false;
                            cx.notify();
                        });
                    });
                });
                task.detach();
            }
            GrpcStreamingType::ClientStreaming => {
                let task = cx.spawn(async move |this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
                    let messages: Vec<String> = vec![message];
                    let result = protide_core::protocols::grpc::execute_client_streaming(
                        &url,
                        &method.full_name,
                        messages,
                        metadata,
                        &proto_path,
                    ).await;

                    match result {
                        Ok(body) => {
                            let body_size = body.len();
                            let _ = cx.update(|cx| {
                                response_panel.update(cx, |panel, cx| {
                                    panel.set_response(ResponseData {
                                        status: 200,
                                        status_text: "OK".to_string(),
                                        headers: vec![
                                            ("content-type".to_string(), "application/grpc+json".to_string()),
                                            ("grpc-status".to_string(), "0".to_string()),
                                        ],
                                        body,
                                        time: std::time::Duration::from_secs(1),
                                        size: body_size,
                                    }, cx);
                                });
                            });
                        }
                        Err(e) => {
                            error!("gRPC client-streaming error: {}", e);
                            let _ = cx.update(|cx| {
                                response_panel.update(cx, |panel, cx| {
                                    panel.set_response(ResponseData {
                                        status: 0,
                                        status_text: "Error".to_string(),
                                        headers: vec![],
                                        body: format!("gRPC Streaming Error: {}", e),
                                        time: std::time::Duration::ZERO,
                                        size: 0,
                                    }, cx);
                                });
                            });
                        }
                    }
                    let _ = cx.update(|cx| {
                        let _ = this.update(cx, |panel, cx| {
                            panel.loading = false;
                            cx.notify();
                        });
                    });
                });
                task.detach();
            }
            GrpcStreamingType::BidiStreaming => {
                let task = cx.spawn(async move |this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
                    let messages: Vec<String> = vec![message];
                    let result = protide_core::protocols::grpc::execute_bidi_streaming(
                        &url,
                        &method.full_name,
                        messages,
                        metadata,
                        &proto_path,
                    ).await;

                    match result {
                        Ok(chunks) => {
                            let body = chunks.join("\n---\n");
                            let body_size = body.len();
                            let _ = cx.update(|cx| {
                                response_panel.update(cx, |panel, cx| {
                                    panel.set_response(ResponseData {
                                        status: 200,
                                        status_text: "OK (bidi)".to_string(),
                                        headers: vec![
                                            ("content-type".to_string(), "application/grpc+json".to_string()),
                                            ("grpc-status".to_string(), "0".to_string()),
                                            ("x-streaming".to_string(), "true".to_string()),
                                        ],
                                        body,
                                        time: std::time::Duration::from_secs(1),
                                        size: body_size,
                                    }, cx);
                                });
                            });
                        }
                        Err(e) => {
                            error!("gRPC bidi-streaming error: {}", e);
                            let _ = cx.update(|cx| {
                                response_panel.update(cx, |panel, cx| {
                                    panel.set_response(ResponseData {
                                        status: 0,
                                        status_text: "Error".to_string(),
                                        headers: vec![],
                                        body: format!("gRPC Bidi Error: {}", e),
                                        time: std::time::Duration::ZERO,
                                        size: 0,
                                    }, cx);
                                });
                            });
                        }
                    }
                    let _ = cx.update(|cx| {
                        let _ = this.update(cx, |panel, cx| {
                            panel.loading = false;
                            cx.notify();
                        });
                    });
                });
                task.detach();
            }
        }
    }

    /// Send a tRPC request
    pub(super) fn send_trpc_request(&mut self, cx: &mut Context<Self>) {
        if self.trpc_procedure.trim().is_empty() {
            return;
        }

        self.loading = true;
        cx.notify();

        let url = self.url.clone();
        let procedure = self.trpc_procedure.clone();
        let params = self.trpc_params_editor.read(cx).content().to_string();

        // Get environment for variable substitution
        let env_state = self.explorer_panel.as_ref().map(|p| p.read(cx).env_state().clone());
        let substitute = |s: &str| -> String {
            if let Some(ref env) = env_state {
                env.substitute(s)
            } else {
                s.to_string()
            }
        };

        let url = substitute(&url);
        let procedure = substitute(&procedure);

        // Collect enabled headers with substitution
        let mut headers: Vec<(String, String)> = self
            .headers
            .iter()
            .filter(|h| h.enabled && !h.key.is_empty())
            .map(|h| (substitute(&h.key), substitute(&h.value)))
            .collect();

        // Add auth headers
        match self.auth_type {
            AuthType::Bearer => {
                if !self.bearer_token.is_empty() {
                    let token = substitute(&self.bearer_token);
                    headers.push(("Authorization".to_string(), format!("Bearer {}", token)));
                }
            }
            AuthType::Basic => {
                if !self.basic_username.is_empty() {
                    let username = substitute(&self.basic_username);
                    let password = substitute(&self.basic_password);
                    let credentials = base64::engine::general_purpose::STANDARD
                        .encode(format!("{}:{}", username, password));
                    headers.push(("Authorization".to_string(), format!("Basic {}", credentials)));
                }
            }
            AuthType::ApiKey => {
                if !self.api_key_name.is_empty() {
                    let key_name = substitute(&self.api_key_name);
                    let key_value = substitute(&self.api_key_value);
                    match self.api_key_location {
                        ApiKeyLocation::Header => {
                            headers.push((key_name, key_value));
                        }
                        ApiKeyLocation::QueryParam => {
                            // For tRPC, query params would be appended to URL
                            // But tRPC typically doesn't use query params, so we'll skip this
                        }
                    }
                }
            }
            AuthType::None => {}
        }

        let response_panel = self.response_panel.clone();

        info!("tRPC {} {}", url, procedure);

        let task = cx.spawn(async move |this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
            let (result_tx, result_rx) = std::sync::mpsc::channel();

            // Spawn blocking thread for HTTP request
            std::thread::spawn(move || {
                let result = protide_core::protocols::trpc::execute_trpc(&url, &procedure, &params, headers);
                let _ = result_tx.send(result);
            });

            // Wait for result
            if let Ok(result) = result_rx.recv_timeout(std::time::Duration::from_secs(30)) {
                match result {
                    Ok((body, elapsed, status)) => {
                        let body_size = body.len();
                        let _ = cx.update(|cx| {
                            response_panel.update(cx, |panel, cx| {
                                panel.set_response(
                                    ResponseData {
                                        status,
                                        status_text: status_text(status).to_string(),
                                        headers: vec![(
                                            "content-type".to_string(),
                                            "application/json".to_string(),
                                        )],
                                        body,
                                        time: elapsed,
                                        size: body_size,
                                    },
                                    cx,
                                );
                            });
                        });
                    }
                    Err(e) => {
                        error!("tRPC error: {}", e);
                        let _ = cx.update(|cx| {
                            response_panel.update(cx, |panel, cx| {
                                let error_body = serde_json::json!({
                                    "error": e,
                                })
                                .to_string();
                                panel.set_response(
                                    ResponseData {
                                        status: 500,
                                        status_text: "tRPC Error".to_string(),
                                        headers: vec![(
                                            "content-type".to_string(),
                                            "application/json".to_string(),
                                        )],
                                        body: error_body.clone(),
                                        time: std::time::Duration::from_secs(0),
                                        size: error_body.len(),
                                    },
                                    cx,
                                );
                            });
                        });
                    }
                }

                // Clear loading state
                let _ = cx.update(|cx| {
                    let _ = this.update(cx, |panel, cx| {
                        panel.loading = false;
                        cx.notify();
                    });
                });
            }
        });
        task.detach();
    }

    fn cursor(&self) -> usize {
        self.url_selection.end
    }

    /// Set body content in the CodeEditor
    pub fn set_body_content(&mut self, content: &str, cx: &mut Context<Self>) {
        self.body_editor.update(cx, |editor, cx| {
            editor.set_content(content, cx);
        });
        self.body = content.to_string();
    }

    /// Set variable extractions from @set annotations
    pub fn set_variable_extractions(&mut self, extractions: Vec<VariableExtraction>, cx: &mut Context<Self>) {
        self.variable_extractions = extractions;
        cx.notify();
    }

    /// Capture a serialisable snapshot of the current editor state.
    /// Accepts any context that derefs to App (Context<MainWindow>, Context<Self>, etc.).
    pub fn capture_draft(&self, cx: &gpui::App) -> crate::session::RequestDraft {
        use crate::session::{HeaderEntry, RequestDraft};
        use AuthType::*;
        use ApiKeyLocation::*;

        RequestDraft {
            protocol: match self.request_mode {
                RequestMode::Http      => "http",
                RequestMode::GraphQL   => "graphql",
                RequestMode::WebSocket => "websocket",
                RequestMode::Grpc      => "grpc",
                RequestMode::Trpc      => "trpc",
                RequestMode::SocketIo  => "socketio",
            }.to_string(),
            active_tab: self.active_tab,
            url: self.url.clone(),
            method: self.method.as_str().to_string(),
            headers: self.headers.iter()
                .filter(|h| !h.key.is_empty())
                .map(|h| HeaderEntry { key: h.key.clone(), value: h.value.clone(), enabled: h.enabled })
                .collect(),
            body: self.body_editor.read(cx).content().to_string(),
            body_type: match self.body_type {
                BodyType::Json   => "json",
                BodyType::Xml    => "xml",
                BodyType::Raw    => "raw",
                BodyType::Form   => "form",
                BodyType::Binary => "binary",
            }.to_string(),
            auth_type: match self.auth_type {
                None    => "none",
                Bearer  => "bearer",
                Basic   => "basic",
                ApiKey  => "apikey",
            }.to_string(),
            bearer_token:    self.bearer_token.clone(),
            basic_username:  self.basic_username.clone(),
            basic_password:  self.basic_password.clone(),
            api_key_name:    self.api_key_name.clone(),
            api_key_value:   self.api_key_value.clone(),
            api_key_location: match self.api_key_location {
                Header     => "header",
                QueryParam => "query",
            }.to_string(),
            graphql_query:          self.graphql_query_editor.read(cx).content().to_string(),
            graphql_variables:      self.graphql_variables_editor.read(cx).content().to_string(),
            graphql_operation_name: self.graphql_operation_name.clone(),
            grpc_message:    self.grpc_message_editor.read(cx).content().to_string(),
            grpc_proto_path: self.grpc_proto_path.clone(),
            grpc_service:    self.grpc_service.clone(),
            grpc_method_name: self.grpc_method.as_ref().map(|m| m.full_name.clone()),
            trpc_procedure:  self.trpc_procedure.clone(),
            trpc_params:     self.trpc_params_editor.read(cx).content().to_string(),
            sio_namespace:   self.sio_namespace.clone(),
            sio_event_name:  self.sio_event_name.clone(),
            sio_payload:     self.sio_payload_editor.read(cx).content().to_string(),
        }
    }

    /// Restore editor state from a previously captured draft.
    pub fn restore_from_draft(&mut self, draft: &crate::session::RequestDraft, cx: &mut Context<Self>) {

        // Switch protocol mode
        self.request_mode = match draft.protocol.as_str() {
            "graphql"   => RequestMode::GraphQL,
            "websocket" => RequestMode::WebSocket,
            "grpc"      => RequestMode::Grpc,
            "trpc"      => RequestMode::Trpc,
            "socketio"  => RequestMode::SocketIo,
            _           => RequestMode::Http,
        };
        self.active_tab = draft.active_tab;
        self.active_edit = Option::None;
        self.method_dropdown_open = false;
        self.variable_extractions.clear();

        // Method + URL
        if let Some(m) = HttpMethod::from_str(&draft.method) {
            self.method = m;
        }
        self.url = draft.url.clone();
        let len = self.url.chars().count();
        self.url_selection = len..len;

        // Headers
        self.headers = draft.headers.iter()
            .map(|h| KeyValuePair { key: h.key.clone(), value: h.value.clone(), enabled: h.enabled })
            .collect();
        if self.headers.is_empty() {
            self.headers.push(KeyValuePair::default());
        } else {
            self.headers.push(KeyValuePair::default());
        }

        // Body
        self.body_type = match draft.body_type.as_str() {
            "xml"    => BodyType::Xml,
            "raw"    => BodyType::Raw,
            "form"   => BodyType::Form,
            "binary" => BodyType::Binary,
            _        => BodyType::Json,
        };
        if !draft.body.is_empty() {
            let b = draft.body.clone();
            self.body_editor.update(cx, |ed, cx| ed.set_content(&b, cx));
        }

        // Auth
        self.auth_type = match draft.auth_type.as_str() {
            "bearer" => AuthType::Bearer,
            "basic"  => AuthType::Basic,
            "apikey" => AuthType::ApiKey,
            _        => AuthType::None,
        };
        self.bearer_token   = draft.bearer_token.clone();
        self.basic_username = draft.basic_username.clone();
        self.basic_password = draft.basic_password.clone();
        self.api_key_name   = draft.api_key_name.clone();
        self.api_key_value  = draft.api_key_value.clone();
        self.api_key_location = match draft.api_key_location.as_str() {
            "query" => ApiKeyLocation::QueryParam,
            _       => ApiKeyLocation::Header,
        };

        // GraphQL
        if !draft.graphql_query.is_empty() {
            let q = draft.graphql_query.clone();
            self.graphql_query_editor.update(cx, |ed, cx| ed.set_content(&q, cx));
        }
        if !draft.graphql_variables.is_empty() {
            let v = draft.graphql_variables.clone();
            self.graphql_variables_editor.update(cx, |ed, cx| ed.set_content(&v, cx));
        }
        self.graphql_operation_name = draft.graphql_operation_name.clone();

        // gRPC
        if !draft.grpc_message.is_empty() {
            let m = draft.grpc_message.clone();
            self.grpc_message_editor.update(cx, |ed, cx| ed.set_content(&m, cx));
        }
        if let Some(ref proto_path) = draft.grpc_proto_path {
            self.load_grpc_proto_from_path(proto_path.clone(), cx);
            if let Some(ref svc) = draft.grpc_service {
                if self.grpc_services.contains(svc) {
                    self.grpc_service = Some(svc.clone());
                    self.grpc_methods.retain(|m| m.full_name.starts_with(svc.as_str()));
                }
            }
            if let Some(ref method_name) = draft.grpc_method_name {
                if let Some(m) = self.grpc_methods.iter().find(|m| &m.full_name == method_name) {
                    self.grpc_method = Some(m.clone());
                }
            }
        }

        // tRPC
        self.trpc_procedure = draft.trpc_procedure.clone();
        if !draft.trpc_params.is_empty() {
            let p = draft.trpc_params.clone();
            self.trpc_params_editor.update(cx, |ed, cx| ed.set_content(&p, cx));
        }

        // Socket.IO
        self.sio_namespace  = draft.sio_namespace.clone();
        self.sio_event_name = draft.sio_event_name.clone();
        if !draft.sio_payload.is_empty() {
            let p = draft.sio_payload.clone();
            self.sio_payload_editor.update(cx, |ed, cx| ed.set_content(&p, cx));
        }

        self.sync_params_from_url(cx);
        cx.notify();
    }

    /// Load request data from a history entry
    pub fn load_from_history(
        &mut self,
        method: String,
        url: String,
        headers: Vec<(String, String)>,
        body: Option<String>,
        cx: &mut Context<Self>,
    ) {
        // Set method
        if let Some(m) = HttpMethod::from_str(&method) {
            self.method = m;
        }

        // Set URL
        self.url = url;
        let char_count = self.url.chars().count();
        self.url_selection = char_count..char_count;

        // Set headers
        self.headers = headers
            .into_iter()
            .map(|(key, value)| KeyValuePair {
                key,
                value,
                enabled: true,
            })
            .collect();
        // Always have at least one empty row
        if self.headers.is_empty() {
            self.headers.push(KeyValuePair::default());
        } else {
            self.headers.push(KeyValuePair::default());
        }

        // Set body
        if let Some(b) = body {
            self.set_body_content(&b, cx);
        }

        // Sync params from URL
        self.sync_params_from_url(cx);

        // Reset editing state
        self.active_edit = None;
        self.method_dropdown_open = false;

        // Clear variable extractions (will be set separately from file load if present)
        self.variable_extractions.clear();

        cx.notify();
    }

    /// Load a parsed request from a .http file, switching protocol as needed.
    pub fn load_from_parsed_request(&mut self, req: &http_parser::Request, cx: &mut Context<Self>) {
        use http_parser::Protocol;

        // Switch mode directly — no URL-override side effects from set_request_mode
        self.request_mode = match req.protocol() {
            Protocol::Http => RequestMode::Http,
            Protocol::GraphQL => RequestMode::GraphQL,
            Protocol::WebSocket => RequestMode::WebSocket,
            Protocol::Grpc => RequestMode::Grpc,
            Protocol::SocketIO => RequestMode::SocketIo,
            Protocol::Trpc => RequestMode::Trpc,
        };
        self.active_tab = 0;
        self.active_edit = None;
        self.method_dropdown_open = false;
        self.variable_extractions.clear();

        // Common headers
        self.headers = req.headers.iter()
            .filter(|h| h.enabled)
            .map(|h| KeyValuePair { key: h.key.clone(), value: h.value.clone(), enabled: true })
            .collect();
        if self.headers.is_empty() {
            self.headers.push(KeyValuePair::default());
        } else {
            self.headers.push(KeyValuePair::default());
        }

        // Protocol-specific field loading — exhaustive, no catch-all
        match req.protocol() {
            Protocol::Http => {
                if let Some(m) = HttpMethod::from_str(req.method.as_str()) {
                    self.method = m;
                }
                self.url = req.url.clone();
                let len = self.url.chars().count();
                self.url_selection = len..len;
                if let Some(body) = &req.body {
                    self.set_body_content(body, cx);
                }
                self.sync_params_from_url(cx);
            }
            Protocol::GraphQL => {
                self.method = HttpMethod::Post;
                self.url = req.url.clone();
                let len = self.url.chars().count();
                self.url_selection = len..len;
                // Body is JSON: { "query": "...", "variables": {...}, "operationName": "..." }
                if let Some(body) = &req.body {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
                        if let Some(query) = json.get("query").and_then(|q| q.as_str()) {
                            let q = query.to_string();
                            self.graphql_query_editor.update(cx, |ed, cx| ed.set_content(&q, cx));
                        }
                        if let Some(vars) = json.get("variables").filter(|v| !v.is_null()) {
                            let v = serde_json::to_string_pretty(vars).unwrap_or_default();
                            self.graphql_variables_editor.update(cx, |ed, cx| ed.set_content(&v, cx));
                        }
                        if let Some(op) = json.get("operationName").and_then(|o| o.as_str()) {
                            self.graphql_operation_name = op.to_string();
                        }
                    }
                }
            }
            Protocol::WebSocket => {
                self.url = req.url.clone();
                let len = self.url.chars().count();
                self.url_selection = len..len;
            }
            Protocol::Grpc => {
                // URL in file: grpc://host:port/Service/Method — strip to server-only part
                let server = req.url.splitn(4, '/').take(3).collect::<Vec<_>>().join("/");
                self.url = server;
                let len = self.url.chars().count();
                self.url_selection = len..len;
                if let Some(body) = &req.body {
                    let b = body.clone();
                    self.grpc_message_editor.update(cx, |ed, cx| ed.set_content(&b, cx));
                }
            }
            Protocol::Trpc => {
                self.method = HttpMethod::Post;
                // URL: http://host/path/trpc/procedure — split into base + procedure
                let url = req.url.as_str();
                if let Some(idx) = url.find("/trpc/") {
                    self.url = url[..idx + 5].to_string(); // up to and including "/trpc"
                    self.trpc_procedure = url[idx + 6..].to_string(); // after "/trpc/"
                } else {
                    self.url = url.to_string();
                }
                let len = self.url.chars().count();
                self.url_selection = len..len;
                if let Some(body) = &req.body {
                    let b = body.clone();
                    self.trpc_params_editor.update(cx, |ed, cx| ed.set_content(&b, cx));
                }
            }
            Protocol::SocketIO => {
                self.url = req.url.clone();
                let len = self.url.chars().count();
                self.url_selection = len..len;
            }
        }

        // Load scripts into editors
        let pre = req.scripts.pre_script.as_deref().unwrap_or("");
        let post = req.scripts.post_script.as_deref().unwrap_or("");
        let tests = req.scripts.tests.as_deref().unwrap_or("");
        self.pre_script_editor.update(cx, |ed, cx| ed.set_content(pre, cx));
        self.post_script_editor.update(cx, |ed, cx| ed.set_content(post, cx));
        self.tests_editor.update(cx, |ed, cx| ed.set_content(tests, cx));
        self.pre_script = pre.to_string();
        self.post_script = post.to_string();
        self.tests = tests.to_string();

        cx.notify();
    }

    fn set_tab(&mut self, index: usize, cx: &mut Context<Self>) {
        self.active_tab = index;
        self.active_edit = None;
        self.edit_selection = 0..0;
        cx.notify();
    }

    fn toggle_method_dropdown(&mut self, cx: &mut Context<Self>) {
        self.method_dropdown_open = !self.method_dropdown_open;
        cx.notify();
    }

    fn select_method(&mut self, method: HttpMethod, cx: &mut Context<Self>) {
        self.method = method;
        self.method_dropdown_open = false;
        cx.notify();
    }

    fn set_body_type(&mut self, body_type: BodyType, cx: &mut Context<Self>) {
        self.body_type = body_type;
        // Update CodeEditor language
        let lang = match body_type {
            BodyType::Json => Language::Json,
            BodyType::Xml  => Language::Xml,
            _              => Language::Plain,
        };
        self.body_editor.update(cx, |ed, cx| ed.set_language(lang, cx));
        // Update Content-Type header
        let content_type = match body_type {
            BodyType::Json   => "application/json",
            BodyType::Xml    => "application/xml",
            BodyType::Form   => "application/x-www-form-urlencoded",
            BodyType::Raw    => "text/plain",
            BodyType::Binary => return cx.notify(), // no content-type update for binary
        };
        if let Some(header) = self.headers.iter_mut().find(|h| h.key.eq_ignore_ascii_case("content-type")) {
            header.value = content_type.to_string();
        } else {
            self.headers.insert(0, KeyValuePair {
                key: "Content-Type".to_string(),
                value: content_type.to_string(),
                enabled: true,
            });
        }
        cx.notify();
    }

    pub(super) fn browse_binary_file(&mut self, cx: &mut Context<Self>) {
        let mut dialog = rfd::FileDialog::new().set_title("Select Binary File");
        if let Some(dir) = last_paths::last_dir("binary_file").or_else(dirs::home_dir) {
            dialog = dialog.set_directory(dir);
        }
        if let Some(path) = dialog.pick_file() {
            last_paths::save_last_dir("binary_file", &path);
            self.binary_file_path = Some(path);
            cx.notify();
        }
    }

    fn toggle_header(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(header) = self.headers.get_mut(index) {
            header.enabled = !header.enabled;
            cx.notify();
        }
    }

    fn add_header(&mut self, cx: &mut Context<Self>) {
        self.headers.push(KeyValuePair::default());
        cx.notify();
    }

    fn remove_header(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.headers.len() && self.headers.len() > 1 {
            self.headers.remove(index);
            // Clear editing if removed row was being edited
            if let Some(target) = self.active_edit {
                match target {
                    EditTarget::HeaderKey(i) | EditTarget::HeaderValue(i) if i == index => {
                        self.active_edit = None;
                    }
                    EditTarget::HeaderKey(i) if i > index => {
                        self.active_edit = Some(EditTarget::HeaderKey(i - 1));
                    }
                    EditTarget::HeaderValue(i) if i > index => {
                        self.active_edit = Some(EditTarget::HeaderValue(i - 1));
                    }
                    _ => {}
                }
            }
            cx.notify();
        }
    }

    fn toggle_grpc_meta(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(meta) = self.grpc_metadata.get_mut(index) {
            meta.enabled = !meta.enabled;
            cx.notify();
        }
    }

    fn add_grpc_meta(&mut self, cx: &mut Context<Self>) {
        self.grpc_metadata.push(KeyValuePair::default());
        cx.notify();
    }

    fn remove_grpc_meta(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.grpc_metadata.len() && self.grpc_metadata.len() > 1 {
            self.grpc_metadata.remove(index);
            if let Some(target) = self.active_edit {
                match target {
                    EditTarget::GrpcMetaKey(i) | EditTarget::GrpcMetaValue(i) if i == index => {
                        self.active_edit = None;
                    }
                    EditTarget::GrpcMetaKey(i) if i > index => {
                        self.active_edit = Some(EditTarget::GrpcMetaKey(i - 1));
                    }
                    EditTarget::GrpcMetaValue(i) if i > index => {
                        self.active_edit = Some(EditTarget::GrpcMetaValue(i - 1));
                    }
                    _ => {}
                }
            }
            cx.notify();
        }
    }

    fn toggle_param(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(param) = self.params.get_mut(index) {
            param.enabled = !param.enabled;
            self.sync_url_from_params(cx);
            cx.notify();
        }
    }

    fn add_param(&mut self, cx: &mut Context<Self>) {
        self.params.push(KeyValuePair::default());
        // Don't sync URL for empty params
        cx.notify();
    }

    fn remove_param(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.params.len() && self.params.len() > 1 {
            self.params.remove(index);
            // Clear editing if removed row was being edited
            if let Some(target) = self.active_edit {
                match target {
                    EditTarget::ParamKey(i) | EditTarget::ParamValue(i) if i == index => {
                        self.active_edit = None;
                    }
                    EditTarget::ParamKey(i) if i > index => {
                        self.active_edit = Some(EditTarget::ParamKey(i - 1));
                    }
                    EditTarget::ParamValue(i) if i > index => {
                        self.active_edit = Some(EditTarget::ParamValue(i - 1));
                    }
                    _ => {}
                }
            }
            self.sync_url_from_params(cx);
            cx.notify();
        }
    }

    fn toggle_form_field(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(field) = self.form_data.get_mut(index) {
            field.enabled = !field.enabled;
            cx.notify();
        }
    }

    fn add_form_field(&mut self, cx: &mut Context<Self>) {
        self.form_data.push(FormField::default());
        cx.notify();
    }

    fn toggle_form_field_type(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(field) = self.form_data.get_mut(index) {
            field.field_type = match field.field_type {
                FormFieldType::Text => FormFieldType::File,
                FormFieldType::File => FormFieldType::Text,
            };
            // Clear file path when switching to text
            if field.field_type == FormFieldType::Text {
                field.file_path = None;
                field.value.clear();
            }
            cx.notify();
        }
    }

    fn select_form_file(&mut self, index: usize, cx: &mut Context<Self>) {
        let mut dialog = rfd::FileDialog::new();
        if let Some(dir) = last_paths::last_dir("form_file") {
            dialog = dialog.set_directory(dir);
        }
        if let Some(path) = dialog.pick_file() {
            last_paths::save_last_dir("form_file", &path);
            if let Some(field) = self.form_data.get_mut(index) {
                field.value = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("file")
                    .to_string();
                field.file_path = Some(path);
                cx.notify();
            }
        }
    }

    fn remove_form_field(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.form_data.len() && self.form_data.len() > 1 {
            self.form_data.remove(index);
            if let Some(target) = self.active_edit {
                match target {
                    EditTarget::FormKey(i) | EditTarget::FormValue(i) if i == index => {
                        self.active_edit = None;
                    }
                    EditTarget::FormKey(i) if i > index => {
                        self.active_edit = Some(EditTarget::FormKey(i - 1));
                    }
                    EditTarget::FormValue(i) if i > index => {
                        self.active_edit = Some(EditTarget::FormValue(i - 1));
                    }
                    _ => {}
                }
            }
            cx.notify();
        }
    }

    fn set_auth_type(&mut self, auth_type: AuthType, cx: &mut Context<Self>) {
        self.auth_type = auth_type;
        self.active_edit = None;
        cx.notify();
    }

    fn toggle_api_key_location(&mut self, cx: &mut Context<Self>) {
        self.api_key_location = match self.api_key_location {
            ApiKeyLocation::Header => ApiKeyLocation::QueryParam,
            ApiKeyLocation::QueryParam => ApiKeyLocation::Header,
        };
        cx.notify();
    }

    // ===== URL <-> Params Sync Methods =====

    /// Get the base URL without query string
    fn get_base_url(&self) -> &str {
        self.url.split('?').next().unwrap_or(&self.url)
    }

    /// Parse query params from URL and update params list
    fn sync_params_from_url(&mut self, cx: &mut Context<Self>) {
        if self.syncing_params {
            return;
        }
        self.syncing_params = true;

        // Find query string
        if let Some(query_start) = self.url.find('?') {
            let query_string = &self.url[query_start + 1..];
            let mut new_params: Vec<KeyValuePair> = Vec::new();

            for pair in query_string.split('&') {
                if pair.is_empty() {
                    continue;
                }
                let mut parts = pair.splitn(2, '=');
                let key = url_decode(parts.next().unwrap_or(""));
                let value = url_decode(parts.next().unwrap_or(""));
                new_params.push(KeyValuePair {
                    key,
                    value,
                    enabled: true,
                });
            }

            if new_params.is_empty() {
                new_params.push(KeyValuePair::default());
            }

            self.params = new_params;
        } else {
            self.params = vec![KeyValuePair::default()];
        }

        // Always show at least 3 rows
        while self.params.len() < 3 {
            self.params.push(KeyValuePair::default());
        }

        self.syncing_params = false;
        cx.notify();
    }

    /// Build URL from base URL and params
    fn sync_url_from_params(&mut self, cx: &mut Context<Self>) {
        if self.syncing_params {
            return;
        }
        self.syncing_params = true;

        let base_url = self.get_base_url().to_string();

        // Build query string from enabled params with non-empty keys
        let query_parts: Vec<String> = self
            .params
            .iter()
            .filter(|p| p.enabled && !p.key.is_empty())
            .map(|p| {
                if p.value.is_empty() {
                    url_encode(&p.key)
                } else {
                    format!("{}={}", url_encode(&p.key), url_encode(&p.value))
                }
            })
            .collect();

        // Update URL
        let old_len = self.url.len();
        if query_parts.is_empty() {
            self.url = base_url;
        } else {
            self.url = format!("{}?{}", base_url, query_parts.join("&"));
        }

        // Adjust cursor if it was beyond the new URL length
        let new_len = self.url.len();
        if self.url_selection.start > new_len {
            self.url_selection.start = new_len;
        }
        if self.url_selection.end > new_len {
            self.url_selection.end = new_len;
        }

        self.syncing_params = false;
        if old_len != new_len {
            cx.notify();
        }
    }

    // ===== Unified Text Editing Methods =====

    /// Get reference to text for an edit target
    fn get_edit_text(&self, target: EditTarget) -> &str {
        match target {
            EditTarget::Url => &self.url,
            EditTarget::HeaderKey(i) => self.headers.get(i).map(|h| h.key.as_str()).unwrap_or(""),
            EditTarget::HeaderValue(i) => self.headers.get(i).map(|h| h.value.as_str()).unwrap_or(""),
            EditTarget::ParamKey(i) => self.params.get(i).map(|p| p.key.as_str()).unwrap_or(""),
            EditTarget::ParamValue(i) => self.params.get(i).map(|p| p.value.as_str()).unwrap_or(""),
            EditTarget::FormKey(i) => self.form_data.get(i).map(|f| f.key.as_str()).unwrap_or(""),
            EditTarget::FormValue(i) => self.form_data.get(i).map(|f| f.value.as_str()).unwrap_or(""),
            EditTarget::Body => &self.body,
            EditTarget::BearerToken => &self.bearer_token,
            EditTarget::BasicUsername => &self.basic_username,
            EditTarget::BasicPassword => &self.basic_password,
            EditTarget::ApiKeyName => &self.api_key_name,
            EditTarget::ApiKeyValue => &self.api_key_value,
            EditTarget::GrpcMetaKey(i) => self.grpc_metadata.get(i).map(|m| m.key.as_str()).unwrap_or(""),
            EditTarget::GrpcMetaValue(i) => self.grpc_metadata.get(i).map(|m| m.value.as_str()).unwrap_or(""),
            EditTarget::TrpcProcedure => &self.trpc_procedure,
            EditTarget::SioNamespace => &self.sio_namespace,
            EditTarget::SioEventName => &self.sio_event_name,
        }
    }

    /// Get mutable reference to text for an edit target
    fn get_edit_text_mut(&mut self, target: EditTarget) -> Option<&mut String> {
        match target {
            EditTarget::Url => Some(&mut self.url),
            EditTarget::HeaderKey(i) => self.headers.get_mut(i).map(|h| &mut h.key),
            EditTarget::HeaderValue(i) => self.headers.get_mut(i).map(|h| &mut h.value),
            EditTarget::ParamKey(i) => self.params.get_mut(i).map(|p| &mut p.key),
            EditTarget::ParamValue(i) => self.params.get_mut(i).map(|p| &mut p.value),
            EditTarget::FormKey(i) => self.form_data.get_mut(i).map(|f| &mut f.key),
            EditTarget::FormValue(i) => self.form_data.get_mut(i).map(|f| &mut f.value),
            EditTarget::Body => Some(&mut self.body),
            EditTarget::BearerToken => Some(&mut self.bearer_token),
            EditTarget::BasicUsername => Some(&mut self.basic_username),
            EditTarget::BasicPassword => Some(&mut self.basic_password),
            EditTarget::ApiKeyName => Some(&mut self.api_key_name),
            EditTarget::ApiKeyValue => Some(&mut self.api_key_value),
            EditTarget::GrpcMetaKey(i) => self.grpc_metadata.get_mut(i).map(|m| &mut m.key),
            EditTarget::GrpcMetaValue(i) => self.grpc_metadata.get_mut(i).map(|m| &mut m.value),
            EditTarget::TrpcProcedure => Some(&mut self.trpc_procedure),
            EditTarget::SioNamespace => Some(&mut self.sio_namespace),
            EditTarget::SioEventName => Some(&mut self.sio_event_name),
        }
    }

    /// Start editing a field
    fn start_editing(&mut self, target: EditTarget, window: &mut Window, cx: &mut Context<Self>) {
        let text_len = self.get_edit_text(target).chars().count();
        self.active_edit = Some(target);
        self.edit_selection = text_len..text_len;
        self.edit_is_selecting = false;
        if matches!(target, EditTarget::Body) {
            self.body_focus.focus(window, cx);
        } else {
            self.edit_focus.focus(window, cx);
            self._edit_blur_sub = Some(cx.on_blur(&self.edit_focus, window, |this, _, cx| {
                this.stop_editing(cx);
            }));
        }
        cx.notify();
    }

    /// Stop editing
    fn stop_editing(&mut self, cx: &mut Context<Self>) {
        self.active_edit = None;
        self.edit_selection = 0..0;
        self.edit_is_selecting = false;
        cx.notify();
    }

    /// Calculate the window X position where text starts for a given edit target
    /// Used for click position calculation since GPUI provides window coordinates
    /// Get cursor position for current edit
    fn edit_cursor(&self) -> usize {
        self.edit_selection.end
    }

    /// Check if there's a selection in current edit
    fn edit_has_selection(&self) -> bool {
        self.edit_selection.start != self.edit_selection.end
    }

    /// Get selected text for current edit
    fn edit_selected_text(&self) -> String {
        if let Some(target) = self.active_edit {
            let text = self.get_edit_text(target);
            let start = self.edit_selection.start.min(self.edit_selection.end);
            let end = self.edit_selection.start.max(self.edit_selection.end);
            text[start..end].to_string()
        } else {
            String::new()
        }
    }

    /// Move cursor to position in current edit (position is char index)
    fn edit_move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        if let Some(target) = self.active_edit {
            let char_count = self.get_edit_text(target).chars().count();
            let offset = offset.min(char_count);
            self.edit_selection = offset..offset;
            cx.notify();
        }
    }

    /// Extend selection to position (position is char index)
    fn edit_select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        if let Some(target) = self.active_edit {
            let char_count = self.get_edit_text(target).chars().count();
            self.edit_selection.end = offset.min(char_count);
            cx.notify();
        }
    }

    /// Select all text in current edit
    fn edit_select_all(&mut self, cx: &mut Context<Self>) {
        if let Some(target) = self.active_edit {
            let char_count = self.get_edit_text(target).chars().count();
            self.edit_selection = 0..char_count;
            cx.notify();
        }
    }

    /// Delete selected text
    fn edit_delete_selection(&mut self, cx: &mut Context<Self>) {
        if self.active_edit.is_some() && self.edit_has_selection() {
            self.save_edit_state();
            self.edit_delete_selection_no_save(cx);
        }
    }

    /// Delete edit selection without saving to undo (used internally)
    fn edit_delete_selection_no_save(&mut self, cx: &mut Context<Self>) {
        if let Some(target) = self.active_edit {
            if self.edit_has_selection() {
                let char_start = self.edit_selection.start.min(self.edit_selection.end);
                let char_end = self.edit_selection.start.max(self.edit_selection.end);
                if let Some(text) = self.get_edit_text_mut(target) {
                    // Convert character indices to byte offsets
                    let byte_start = char_to_byte_offset(text, char_start);
                    let byte_end = char_to_byte_offset(text, char_end);
                    text.replace_range(byte_start..byte_end, "");
                    self.edit_selection = char_start..char_start;
                    // Sync URL <-> params
                    self.sync_after_edit(target, cx);
                    cx.notify();
                }
            }
        }
    }

    /// Insert text at cursor (replacing selection if any)
    /// Save edit state to undo stack before making changes
    fn save_edit_state(&mut self) {
        if let Some(target) = self.active_edit {
            let text = self.get_edit_text(target).to_string();
            self.edit_undo_stack.push((target, text, self.edit_selection.clone()));
            if self.edit_undo_stack.len() > 100 {
                self.edit_undo_stack.remove(0);
            }
            self.edit_redo_stack.clear();
        }
    }

    /// Undo edit change
    fn edit_undo(&mut self, cx: &mut Context<Self>) {
        if let Some((target, text, selection)) = self.edit_undo_stack.pop() {
            // Save current state to redo
            let current_text = self.get_edit_text(target).to_string();
            self.edit_redo_stack.push((target, current_text, self.edit_selection.clone()));

            // Restore previous state
            if let Some(field) = self.get_edit_text_mut(target) {
                *field = text;
            }
            self.edit_selection = selection;
            self.sync_after_edit(target, cx);
            cx.notify();
        }
    }

    /// Redo edit change
    fn edit_redo(&mut self, cx: &mut Context<Self>) {
        if let Some((target, text, selection)) = self.edit_redo_stack.pop() {
            // Save current state to undo
            let current_text = self.get_edit_text(target).to_string();
            self.edit_undo_stack.push((target, current_text, self.edit_selection.clone()));

            // Restore redo state
            if let Some(field) = self.get_edit_text_mut(target) {
                *field = text;
            }
            self.edit_selection = selection;
            self.sync_after_edit(target, cx);
            cx.notify();
        }
    }

    fn edit_insert_text(&mut self, insert: &str, cx: &mut Context<Self>) {
        if let Some(target) = self.active_edit {
            self.save_edit_state();
            self.edit_delete_selection_no_save(cx);
            let char_pos = self.edit_selection.start;
            if let Some(text) = self.get_edit_text_mut(target) {
                let byte_pos = char_to_byte_offset(text, char_pos);
                text.insert_str(byte_pos, insert);
                let new_char_pos = char_pos + insert.chars().count();
                self.edit_selection = new_char_pos..new_char_pos;
                self.sync_after_edit(target, cx);
                cx.notify();
            }
            // Auto-enable and auto-grow KV rows after text insertion
            match target {
                EditTarget::ParamKey(i) | EditTarget::ParamValue(i) => {
                    let (key_empty, val_empty) = self.params.get(i)
                        .map_or((true, true), |p| (p.key.is_empty(), p.value.is_empty()));
                    if let Some(param) = self.params.get_mut(i) {
                        if !param.enabled && (!key_empty || !val_empty) {
                            param.enabled = true;
                            self.sync_url_from_params(cx);
                        }
                    }
                    if i + 1 == self.params.len()
                        && self.params.last().map_or(false, |p| !p.key.is_empty() || !p.value.is_empty())
                    {
                        self.params.push(KeyValuePair::default());
                        cx.notify();
                    }
                }
                EditTarget::HeaderKey(i) | EditTarget::HeaderValue(i) => {
                    let (key_empty, val_empty) = self.headers.get(i)
                        .map_or((true, true), |h| (h.key.is_empty(), h.value.is_empty()));
                    if let Some(header) = self.headers.get_mut(i) {
                        if !header.enabled && (!key_empty || !val_empty) {
                            header.enabled = true;
                        }
                    }
                    if i + 1 == self.headers.len()
                        && self.headers.last().map_or(false, |h| !h.key.is_empty() || !h.value.is_empty())
                    {
                        self.headers.push(KeyValuePair::default());
                        cx.notify();
                    }
                }
                _ => {}
            }
        }
    }

    /// Sync URL and params after editing
    fn sync_after_edit(&mut self, target: EditTarget, cx: &mut Context<Self>) {
        match target {
            EditTarget::ParamKey(_) | EditTarget::ParamValue(_) => {
                self.sync_url_from_params(cx);
            }
            EditTarget::Url => {
                self.sync_params_from_url(cx);
            }
            _ => {}
        }
    }

    /// Calculate character index from x position (for single-line fields)
    /// Returns a character index (not byte offset)
    fn edit_index_for_x(&self, x: f32, char_width: f32) -> usize {
        if let Some(target) = self.active_edit {
            let char_count = self.get_edit_text(target).chars().count();
            if x <= 0.0 {
                0
            } else {
                let approx_char = (x / char_width) as usize;
                approx_char.min(char_count)
            }
        } else {
            0
        }
    }

    /// Handle mouse down for single-line edit fields
    /// Calculates click position based on the target's window position
    fn handle_edit_mouse_down(&mut self, event: &MouseDownEvent, target: EditTarget, char_width: f32, cx: &mut Context<Self>) {
        self.edit_is_selecting = true;
        // Use canvas-captured origin (text content start in window coords)
        let text_start_x = self.edit_input_origins.get(&target).copied().unwrap_or(0.0);
        let click_x = (f32::from(event.position.x) - text_start_x).max(0.0);
        let index = self.edit_index_for_x(click_x.max(0.0), char_width);

        // Cycle: 1=cursor, 2=word, 3=all, 4+=cursor
        let effective_click = if event.click_count >= 4 { 1 } else { event.click_count };

        match effective_click {
            2 => {
                // Double-click: select word
                if let Some(target) = self.active_edit {
                    let text = self.get_edit_text(target);
                    let start = find_word_start(&text, index);
                    let end = find_word_end(&text, index);
                    self.edit_selection = start..end;
                    cx.notify();
                }
            }
            3 => {
                // Triple-click: select all
                self.edit_select_all(cx);
            }
            _ => {
                // Single click (or 4th+ click to deselect)
                if event.modifiers.shift {
                    self.edit_select_to(index, cx);
                } else {
                    self.edit_move_to(index, cx);
                }
            }
        }
    }

    /// Calculate cursor position after moving up one line
    fn body_cursor_up(&self) -> usize {
        let text = &self.body;
        let cursor = self.edit_cursor();

        if text.is_empty() || cursor == 0 {
            return 0;
        }

        // Find current line start
        let current_line_start = text[..cursor].rfind('\n').map(|i| i + 1).unwrap_or(0);

        // If already on first line, go to start
        if current_line_start == 0 {
            return 0;
        }

        // Column in current line
        let col = cursor - current_line_start;

        // Find previous line start (line before current)
        let prev_line_end = current_line_start - 1; // newline char position
        let prev_line_start = text[..prev_line_end].rfind('\n').map(|i| i + 1).unwrap_or(0);
        let prev_line_len = prev_line_end - prev_line_start;

        // Move to same column in previous line (or end of line if shorter)
        prev_line_start + col.min(prev_line_len)
    }

    /// Calculate cursor position after moving down one line
    fn body_cursor_down(&self) -> usize {
        let text = &self.body;
        let cursor = self.edit_cursor();

        if text.is_empty() {
            return 0;
        }

        // Find current line start
        let current_line_start = text[..cursor].rfind('\n').map(|i| i + 1).unwrap_or(0);

        // Column in current line
        let col = cursor - current_line_start;

        // Find next line start
        let Some(newline_pos) = text[cursor..].find('\n') else {
            // No more lines, go to end
            return text.len();
        };

        let next_line_start = cursor + newline_pos + 1;

        // Find next line end
        let next_line_end = text[next_line_start..].find('\n')
            .map(|i| next_line_start + i)
            .unwrap_or(text.len());
        let next_line_len = next_line_end - next_line_start;

        // Move to same column in next line (or end of line if shorter)
        next_line_start + col.min(next_line_len)
    }

    /// Handle mouse move for single-line edit fields
    fn handle_edit_mouse_move(&mut self, event: &MouseMoveEvent, char_width: f32, cx: &mut Context<Self>) {
        if self.edit_is_selecting {
            let text_start_x = self.active_edit
                .and_then(|t| self.edit_input_origins.get(&t).copied())
                .unwrap_or(0.0);
            let click_x = f32::from(event.position.x) - text_start_x;
            let index = self.edit_index_for_x(click_x.max(0.0), char_width);
            if self.edit_selection.end != index {
                self.edit_selection.end = index;
                cx.notify();
            }
        }
    }

    /// Handle mouse up for edit fields
    fn handle_edit_mouse_up(&mut self, _event: &MouseUpEvent, _cx: &mut Context<Self>) {
        self.edit_is_selecting = false;
    }

    /// Unified key handler for all edit fields
    fn handle_edit_key(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let Some(target) = self.active_edit else {
            return;
        };

        let key = event.keystroke.key.as_str();
        let ctrl = event.keystroke.modifiers.control;
        let shift = event.keystroke.modifiers.shift;
        let is_body = matches!(target, EditTarget::Body);

        // Handle Ctrl shortcuts
        if ctrl {
            match key {
                "a" => {
                    self.edit_select_all(cx);
                    return;
                }
                "c" => {
                    if self.edit_has_selection() {
                        cx.write_to_clipboard(ClipboardItem::new_string(self.edit_selected_text()));
                    }
                    return;
                }
                "x" => {
                    if self.edit_has_selection() {
                        cx.write_to_clipboard(ClipboardItem::new_string(self.edit_selected_text()));
                        self.edit_delete_selection(cx);
                    }
                    return;
                }
                "v" => {
                    if let Some(item) = cx.read_from_clipboard() {
                        if let Some(text) = item.text() {
                            let insert_text = if is_body {
                                text.to_string()
                            } else {
                                text.replace('\n', "")
                            };
                            self.edit_insert_text(&insert_text, cx);
                        }
                    }
                    return;
                }
                "z" => {
                    if shift {
                        // Ctrl+Shift+Z = Redo
                        self.edit_redo(cx);
                    } else {
                        // Ctrl+Z = Undo
                        self.edit_undo(cx);
                    }
                    return;
                }
                "y" => {
                    // Ctrl+Y = Redo (alternative)
                    self.edit_redo(cx);
                    return;
                }
                _ => {}
            }
        }

        match key {
            "left" => {
                if shift {
                    if self.edit_selection.end > 0 {
                        self.edit_selection.end -= 1;
                        cx.notify();
                    }
                } else if self.edit_has_selection() {
                    let start = self.edit_selection.start.min(self.edit_selection.end);
                    self.edit_move_to(start, cx);
                } else if self.edit_cursor() > 0 {
                    self.edit_move_to(self.edit_cursor() - 1, cx);
                }
            }
            "right" => {
                let char_count = self.get_edit_text(target).chars().count();
                if shift {
                    if self.edit_selection.end < char_count {
                        self.edit_selection.end += 1;
                        cx.notify();
                    }
                } else if self.edit_has_selection() {
                    let end = self.edit_selection.start.max(self.edit_selection.end);
                    self.edit_move_to(end, cx);
                } else if self.edit_cursor() < char_count {
                    self.edit_move_to(self.edit_cursor() + 1, cx);
                }
            }
            "home" => {
                if shift {
                    self.edit_selection.end = 0;
                    cx.notify();
                } else {
                    self.edit_move_to(0, cx);
                }
            }
            "end" => {
                let char_count = self.get_edit_text(target).chars().count();
                if shift {
                    self.edit_selection.end = char_count;
                    cx.notify();
                } else {
                    self.edit_move_to(char_count, cx);
                }
            }
            "up" => {
                if is_body {
                    let new_pos = self.body_cursor_up();
                    if shift {
                        self.edit_selection.end = new_pos;
                        cx.notify();
                    } else {
                        self.edit_move_to(new_pos, cx);
                    }
                }
            }
            "down" => {
                if is_body {
                    let new_pos = self.body_cursor_down();
                    if shift {
                        self.edit_selection.end = new_pos;
                        cx.notify();
                    } else {
                        self.edit_move_to(new_pos, cx);
                    }
                }
            }
            "backspace" => {
                if self.edit_has_selection() {
                    self.edit_delete_selection(cx);
                } else if self.edit_cursor() > 0 {
                    self.save_edit_state();
                    let char_pos = self.edit_cursor() - 1;
                    if let Some(text) = self.get_edit_text_mut(target) {
                        // Convert char position to byte offset and remove that char
                        let byte_pos = char_to_byte_offset(text, char_pos);
                        let next_byte_pos = char_to_byte_offset(text, char_pos + 1);
                        text.replace_range(byte_pos..next_byte_pos, "");
                        self.edit_selection = char_pos..char_pos;
                        self.sync_after_edit(target, cx);
                        cx.notify();
                    }
                }
            }
            "delete" => {
                let char_count = self.get_edit_text(target).chars().count();
                if self.edit_has_selection() {
                    self.edit_delete_selection(cx);
                } else {
                    let cursor = self.edit_cursor();
                    if cursor < char_count {
                        self.save_edit_state();
                        if let Some(text) = self.get_edit_text_mut(target) {
                            // Convert char position to byte offset and remove that char
                            let byte_pos = char_to_byte_offset(text, cursor);
                            let next_byte_pos = char_to_byte_offset(text, cursor + 1);
                            text.replace_range(byte_pos..next_byte_pos, "");
                            self.sync_after_edit(target, cx);
                            cx.notify();
                        }
                    }
                }
            }
            "escape" => {
                self.stop_editing(cx);
            }
            "enter" => {
                if is_body {
                    self.edit_insert_text("\n", cx);
                } else {
                    // Move to next field in kv pairs
                    self.move_to_next_field(cx);
                }
            }
            "tab" => {
                if is_body {
                    self.edit_insert_text("  ", cx);
                } else {
                    self.move_to_next_field(cx);
                }
            }
            _ => {
                // Handle printable characters
                if let Some(ch) = &event.keystroke.key_char {
                    self.edit_insert_text(ch, cx);
                }
            }
        }
    }

    /// Move to next field (for tab/enter in kv editors)
    fn move_to_next_field(&mut self, cx: &mut Context<Self>) {
        let Some(target) = self.active_edit else {
            return;
        };

        let next_target = match target {
            EditTarget::HeaderKey(i) => Some(EditTarget::HeaderValue(i)),
            EditTarget::HeaderValue(i) => {
                if i + 1 < self.headers.len() {
                    Some(EditTarget::HeaderKey(i + 1))
                } else {
                    None
                }
            }
            EditTarget::ParamKey(i) => Some(EditTarget::ParamValue(i)),
            EditTarget::ParamValue(i) => {
                if i + 1 < self.params.len() {
                    Some(EditTarget::ParamKey(i + 1))
                } else {
                    None
                }
            }
            EditTarget::FormKey(i) => Some(EditTarget::FormValue(i)),
            EditTarget::FormValue(i) => {
                if i + 1 < self.form_data.len() {
                    Some(EditTarget::FormKey(i + 1))
                } else {
                    None
                }
            }
            _ => None,
        };

        if let Some(next) = next_target {
            let text_len = self.get_edit_text(next).len();
            self.active_edit = Some(next);
            self.edit_selection = text_len..text_len;
            cx.notify();
        } else {
            self.stop_editing(cx);
        }
    }

    /// Save the current request to a .http file
    pub fn save_request(&mut self, cx: &mut Context<Self>) {
        let content = self.generate_http_content(cx);

        // Save in-place if a file is already loaded
        if let Some(ref path) = self.current_file.clone() {
            if let Err(e) = std::fs::write(path, &content) {
                error!("Failed to save request {}: {}", path.display(), e);
            } else {
                info!("Saved: {}", path.display());
                self.save_feedback = true;
                cx.notify();
                cx.spawn(async move |this, cx| {
                    cx.background_executor().timer(std::time::Duration::from_millis(1500)).await;
                    this.update(cx, |this, cx| {
                        this.save_feedback = false;
                        cx.notify();
                    }).ok();
                }).detach();
            }
            return;
        }

        // Otherwise open save dialog
        let default_name = if self.url.is_empty() {
            "new-request.http".to_string()
        } else {
            let name = self.url.split('/')
                .filter(|s| !s.is_empty() && !s.contains("://") && !s.contains('.'))
                .last()
                .unwrap_or("request");
            format!("{}.http", name)
        };

        let start_dir = last_paths::last_dir("save_request").or_else(dirs::home_dir);
        let mut dialog = rfd::FileDialog::new()
            .set_title("Save Request")
            .set_file_name(&default_name)
            .add_filter("HTTP Request", &["http"]);
        if let Some(dir) = start_dir {
            dialog = dialog.set_directory(dir);
        }

        if let Some(path) = dialog.save_file() {
            last_paths::save_last_dir("save_request", &path);
            let path = if path.extension().map_or(true, |ext| ext != "http") {
                path.with_extension("http")
            } else {
                path
            };
            if let Err(e) = std::fs::write(&path, &content) {
                error!("Failed to save request {}: {}", path.display(), e);
            } else {
                info!("Saved: {}", path.display());
                self.current_file = Some(path);
            }
        }
    }

    /// Generate .http file content from current request state
    fn generate_http_content(&self, cx: &Context<Self>) -> String {
        let mut lines = Vec::new();

        // Request name comment
        let name = if self.url.is_empty() {
            "New Request"
        } else {
            &self.url
        };
        lines.push(format!("### {}", name));
        lines.push(String::new());

        // Proto file annotation (gRPC)
        if let Some(ref proto_path) = self.grpc_proto_path {
            lines.push(format!("# @proto {}", proto_path.display()));
        }

        // Method and URL
        lines.push(format!("{} {}", self.method.as_str(), self.url));

        // Headers
        for header in &self.headers {
            if header.enabled && !header.key.is_empty() {
                lines.push(format!("{}: {}", header.key, header.value));
            }
        }

        // Auth headers
        match self.auth_type {
            AuthType::None => {}
            AuthType::Bearer => {
                if !self.bearer_token.is_empty() {
                    lines.push(format!("Authorization: Bearer {}", self.bearer_token));
                }
            }
            AuthType::Basic => {
                if !self.basic_username.is_empty() || !self.basic_password.is_empty() {
                    // Base64 encoding for username:password
                    let credentials = format!("{}:{}", self.basic_username, self.basic_password);
                    let encoded = base64_encode(credentials.as_bytes());
                    lines.push(format!("Authorization: Basic {}", encoded));
                }
            }
            AuthType::ApiKey => {
                if !self.api_key_name.is_empty() && !self.api_key_value.is_empty() {
                    if self.api_key_location == ApiKeyLocation::Header {
                        lines.push(format!("{}: {}", self.api_key_name, self.api_key_value));
                    }
                    // Query params are added to URL, handled separately
                }
            }
        }

        // Body - get from CodeEditor
        let body_content = self.body_editor.read(cx).content().to_string();
        if !body_content.is_empty() {
            lines.push(String::new());
            lines.push(body_content);
        }

        lines.join("\n")
    }

    /// Generate code for current request using selected language
    pub fn generate_code(&mut self, language: CodegenLanguage, cx: &mut Context<Self>) {
        // Build CodegenRequest from current state
        let mut headers: Vec<(String, String)> = self
            .headers
            .iter()
            .filter(|h| h.enabled && !h.key.is_empty())
            .map(|h| (h.key.clone(), h.value.clone()))
            .collect();

        // Add auth headers
        match self.auth_type {
            AuthType::Bearer if !self.bearer_token.is_empty() => {
                headers.push(("Authorization".to_string(), format!("Bearer {}", self.bearer_token)));
            }
            AuthType::Basic if !self.basic_username.is_empty() || !self.basic_password.is_empty() => {
                let credentials = format!("{}:{}", self.basic_username, self.basic_password);
                let encoded = base64_encode(credentials.as_bytes());
                headers.push(("Authorization".to_string(), format!("Basic {}", encoded)));
            }
            AuthType::ApiKey if !self.api_key_name.is_empty() && self.api_key_location == ApiKeyLocation::Header => {
                headers.push((self.api_key_name.clone(), self.api_key_value.clone()));
            }
            _ => {}
        }

        // Get body content
        let body = self.body_editor.read(cx).content().to_string();
        let body = if body.trim().is_empty() { None } else { Some(body) };

        let request = CodegenRequest {
            method: self.method.as_str().to_string(),
            url: self.url.clone(),
            headers,
            body,
        };

        let code = codegen::generate(&request, language);
        self.codegen_language = language;
        self.codegen_content = Some(code.clone());
        let editor_lang = match language {
            CodegenLanguage::Curl       => Language::Shell,
            CodegenLanguage::Python     => Language::Python,
            CodegenLanguage::JavaScript => Language::JavaScript,
            CodegenLanguage::Go         => Language::Go,
            CodegenLanguage::Rust       => Language::Rust,
        };
        self.codegen_editor.update(cx, |editor, cx| {
            editor.set_content(&code, cx);
            editor.set_language(editor_lang, cx);
        });
        cx.notify();
    }

    /// Close code modal
    pub fn codegen_lang_name(&self) -> &'static str {
        match self.codegen_language {
            CodegenLanguage::Curl => "cURL",
            CodegenLanguage::Python => "Python",
            CodegenLanguage::JavaScript => "JavaScript",
            CodegenLanguage::Go => "Go",
            CodegenLanguage::Rust => "Rust",
        }
    }

    pub fn close_codegen_panel(&mut self, cx: &mut Context<Self>) {
        self.codegen_content = None;
        cx.notify();
    }

    /// Copy generated code to clipboard
    pub fn copy_generated_code(&self, cx: &mut Context<Self>) {
        if let Some(code) = &self.codegen_content {
            cx.write_to_clipboard(ClipboardItem::new_string(code.clone()));
        }
    }

    /// Open import modal
    pub fn open_import_modal(&mut self, cx: &mut Context<Self>) {
        self.import_modal_open = true;
        self.import_text.clear();
        self.import_error = None;
        self.import_editor.update(cx, |ed, cx| ed.set_content("", cx));
        cx.notify();
    }

    /// Close import modal
    pub fn close_import_modal(&mut self, cx: &mut Context<Self>) {
        self.import_modal_open = false;
        self.import_text.clear();
        self.import_error = None;
        cx.notify();
    }

    /// Update import text
    pub(super) fn set_import_text(&mut self, text: String, cx: &mut Context<Self>) {
        self.import_editor.update(cx, |ed, cx| ed.set_content(&text, cx));
        self.import_text = text;
        self.import_error = None;
        cx.notify();
    }

    /// Browse for a file to import (Postman collection, Bruno, OpenAPI, etc.)
    pub(super) fn browse_import_file(&mut self, cx: &mut Context<Self>) {
        let start_dir = last_paths::last_dir("import_collection")
            .or_else(dirs::home_dir);
        let mut dialog = rfd::FileDialog::new()
            .set_title("Import Collection")
            .add_filter("All Supported", &["json", "yaml", "yml", "bru", "txt", "curl"])
            .add_filter("Postman Collection", &["json"])
            .add_filter("OpenAPI/Swagger", &["json", "yaml", "yml"])
            .add_filter("Bruno Collection", &["bru"])
            .add_filter("cURL Command", &["txt", "curl"])
            .add_filter("All Files", &["*"]);

        if let Some(dir) = start_dir {
            dialog = dialog.set_directory(dir);
        }

        if let Some(path) = dialog.pick_file() {
            last_paths::save_last_dir("import_collection", &path);
            match std::fs::read_to_string(&path) {
                Ok(content) => {
                    self.set_import_text(content, cx);
                }
                Err(e) => {
                    self.import_error = Some(format!("Failed to read file: {}", e));
                }
            }
        }
        cx.notify();
    }

    /// Execute import from the current import_text
    pub(super) fn execute_import(&mut self, cx: &mut Context<Self>) {
        // Sync editor content into import_text before parsing
        let editor_content = self.import_editor.read(cx).content().to_string();
        if !editor_content.is_empty() {
            self.import_text = editor_content;
        }
        if self.import_text.trim().is_empty() {
            self.import_error = Some("Please paste a cURL command or request data".to_string());
            cx.notify();
            return;
        }

        match import::import(&self.import_text) {
            Ok(result) => {
                if let Some(request) = result.requests.into_iter().next() {
                    // Convert http_parser::HttpMethod to our HttpMethod
                    let method = match request.method {
                        http_parser::HttpMethod::Get => HttpMethod::Get,
                        http_parser::HttpMethod::Post => HttpMethod::Post,
                        http_parser::HttpMethod::Put => HttpMethod::Put,
                        http_parser::HttpMethod::Patch => HttpMethod::Patch,
                        http_parser::HttpMethod::Delete => HttpMethod::Delete,
                        // Map unsupported methods to GET
                        _ => HttpMethod::Get,
                    };

                    // Load the imported request
                    self.method = method;
                    self.url = request.url;
                    self.url_selection = self.url.len()..self.url.len();

                    // Convert headers (preserve enabled state from import)
                    self.headers = request.headers
                        .into_iter()
                        .map(|h| KeyValuePair {
                            key: h.key,
                            value: h.value,
                            enabled: h.enabled,
                        })
                        .collect();
                    // Add empty row for new headers
                    self.headers.push(KeyValuePair::default());

                    // Set body
                    if let Some(body) = request.body {
                        self.body = body.clone();
                        self.body_editor.update(cx, |editor, cx| {
                            editor.set_content(&body, cx);
                        });
                        // Detect body type from Content-Type header
                        let is_json = self.headers.iter().any(|h| {
                            h.key.eq_ignore_ascii_case("content-type") && h.value.contains("json")
                        });
                        self.body_type = if is_json { BodyType::Json } else { BodyType::Raw };
                    }

                    // Close modal on success
                    self.import_modal_open = false;
                    self.import_text.clear();
                    self.import_error = None;
                } else {
                    self.import_error = Some("No request found in import data".to_string());
                }
            }
            Err(e) => {
                self.import_error = Some(e);
            }
        }
        cx.notify();
    }

    pub fn send_request(&mut self, cx: &mut Context<Self>) {
        if self.loading || self.url.is_empty() {
            return;
        }

        self.loading = true;
        cx.notify();

        // Get body content from CodeEditor
        let body_content = self.body_editor.read(cx).content().to_string();

        // Get GraphQL content if in GraphQL mode
        let is_graphql_mode = self.request_mode == RequestMode::GraphQL;
        let graphql_query = if is_graphql_mode {
            self.graphql_query_editor.read(cx).content().to_string()
        } else {
            String::new()
        };
        let graphql_variables = if is_graphql_mode {
            self.graphql_variables_editor.read(cx).content().to_string()
        } else {
            String::new()
        };

        // Get script content from editors
        let pre_script = self.pre_script_editor.read(cx).content().to_string();
        let post_script = self.post_script_editor.read(cx).content().to_string();
        let tests_script = self.tests_editor.read(cx).content().to_string();

        // Set response panel to loading
        self.response_panel.update(cx, |panel, cx| {
            panel.set_loading(cx);
        });

        // Get environment state for variable substitution
        let env_state = self.explorer_panel.as_ref().map(|panel| {
            panel.read(cx).env_state().clone()
        });

        // Helper closure to substitute variables
        let substitute = |s: &str| -> String {
            if let Some(ref env) = env_state {
                env.substitute(s)
            } else {
                s.to_string()
            }
        };

        // Substitute variables in URL
        let url = substitute(&self.url);
        let method = self.method.clone();
        let response_panel = self.response_panel.clone();
        let variable_extractions = self.variable_extractions.clone();
        let explorer_panel = self.explorer_panel.clone();

        // Substitute variables in headers
        let mut headers: Vec<(String, String)> = self
            .headers
            .iter()
            .filter(|h| h.enabled && !h.key.is_empty())
            .map(|h| (substitute(&h.key), substitute(&h.value)))
            .collect();

        // Add authentication headers (with variable substitution)
        let auth_type = self.auth_type;
        let bearer_token = substitute(&self.bearer_token);
        let basic_username = substitute(&self.basic_username);
        let basic_password = substitute(&self.basic_password);
        let api_key_name = substitute(&self.api_key_name);
        let api_key_value = substitute(&self.api_key_value);
        let api_key_location = self.api_key_location;

        match auth_type {
            AuthType::None => {}
            AuthType::Bearer => {
                if !bearer_token.is_empty() {
                    headers.push(("Authorization".to_string(), format!("Bearer {}", bearer_token)));
                }
            }
            AuthType::Basic => {
                if !basic_username.is_empty() || !basic_password.is_empty() {
                    let credentials = format!("{}:{}", basic_username, basic_password);
                    let encoded = base64_encode(credentials.as_bytes());
                    headers.push(("Authorization".to_string(), format!("Basic {}", encoded)));
                }
            }
            AuthType::ApiKey => {
                if !api_key_name.is_empty() && !api_key_value.is_empty() {
                    if api_key_location == ApiKeyLocation::Header {
                        headers.push((api_key_name.clone(), api_key_value.clone()));
                    }
                    // Query param will be handled in URL
                }
            }
        }

        let binary_file_path = self.binary_file_path.clone();

        // Substitute variables in body and collect form data
        let has_files = self.body_type == BodyType::Form
            && self.form_data.iter().any(|f| f.enabled && f.field_type == FormFieldType::File && f.file_path.is_some());

        // Collect form fields for multipart (must be done before thread spawn)
        let form_fields: Vec<(String, String, Option<std::path::PathBuf>, bool)> = if self.body_type == BodyType::Form {
            self.form_data
                .iter()
                .filter(|f| f.enabled && !f.key.is_empty())
                .map(|f| (
                    substitute(&f.key),
                    substitute(&f.value),
                    f.file_path.clone(),
                    f.field_type == FormFieldType::File,
                ))
                .collect()
        } else {
            Vec::new()
        };

        let exec_body = if is_graphql_mode {
            ExecutionBody::None // body is constructed by run_http for GraphQL
        } else if matches!(method, HttpMethod::Post | HttpMethod::Put | HttpMethod::Patch | HttpMethod::Custom(_)) {
            match self.body_type {
                BodyType::Form if !has_files => {
                    let s = form_fields
                        .iter()
                        .filter(|(_, _, _, is_file)| !is_file)
                        .map(|(k, v, _, _)| format!("{}={}", url_encode(k), url_encode(v)))
                        .collect::<Vec<_>>()
                        .join("&");
                    if s.is_empty() { ExecutionBody::None } else { ExecutionBody::Text(s) }
                }
                BodyType::Form => ExecutionBody::Multipart(
                    form_fields
                        .iter()
                        .map(|(k, v, path, is_file)| FormPart {
                            name: k.clone(),
                            value: if *is_file {
                                FormPartValue::File(path.clone().unwrap_or_default())
                            } else {
                                FormPartValue::Text(v.clone())
                            },
                        })
                        .collect(),
                ),
                BodyType::Binary => binary_file_path
                    .as_ref()
                    .and_then(|p| std::fs::read(p).ok())
                    .map(ExecutionBody::Binary)
                    .unwrap_or(ExecutionBody::None),
                _ => ExecutionBody::Text(substitute(&body_content)),
            }
        } else {
            ExecutionBody::None
        };

        let exec_mode = if is_graphql_mode {
            ExecutionMode::GraphQL {
                query: substitute(&graphql_query),
                variables: substitute(&graphql_variables),
                operation_name: if self.graphql_operation_name.trim().is_empty() {
                    None
                } else {
                    Some(substitute(&self.graphql_operation_name))
                },
            }
        } else {
            ExecutionMode::Http
        };

        let env_vars: std::collections::HashMap<String, String> = env_state
            .as_ref()
            .and_then(|e| e.active())
            .map(|env| env.variables.clone())
            .unwrap_or_default();

        // Build final URL with API key as query param if needed
        let final_url = if auth_type == AuthType::ApiKey
            && api_key_location == ApiKeyLocation::QueryParam
            && !api_key_name.is_empty()
            && !api_key_value.is_empty()
        {
            if url.contains('?') {
                format!("{}&{}={}", url, api_key_name, api_key_value)
            } else {
                format!("{}?{}={}", url, api_key_name, api_key_value)
            }
        } else {
            url
        };

        // Add to history
        let history_badge = match self.request_mode {
            RequestMode::Http => method.as_str().to_string(),
            RequestMode::GraphQL => "GQL".to_string(),
            RequestMode::WebSocket => "WS".to_string(),
            RequestMode::Grpc => "GRPC".to_string(),
            RequestMode::Trpc => "TRPC".to_string(),
            RequestMode::SocketIo => "SIO".to_string(),
        };
        let history_id = cx.update_global::<super::history::RequestHistory, _>(|history, _| {
            history.add(
                history_badge,
                final_url.clone(),
                headers.clone(),
                exec_body.as_text(),
            )
        });

        let console_panel = self.console_panel.clone();
        let log_protocol = match self.request_mode {
            RequestMode::Http     => "HTTP",
            RequestMode::GraphQL  => "GraphQL",
            RequestMode::WebSocket => "WebSocket",
            RequestMode::Grpc     => "gRPC",
            RequestMode::Trpc     => "tRPC",
            RequestMode::SocketIo => "Socket.IO",
        };
        let log_method = method.as_str().to_string();

        let req = ExecutionRequest {
            method: method.as_str().to_string(),
            url: final_url.clone(),
            headers,
            body: exec_body,
            mode: exec_mode,
            pre_script,
            post_script,
            tests: tests_script,
            env_vars,
            variable_extractions,
        };

        let log_url = final_url.clone();
        info!("[{}] → {} {}", log_protocol, log_method, log_url);

        cx.spawn(async move |this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
            let result = std::thread::spawn(move || protide_core::execution::execute(req))
                .join()
                .unwrap_or_else(|_| Err("Request thread panicked".to_string()));

            match result {
                Ok(data) => {
                    info!("[{}] ← {} {} in {}ms", log_protocol, data.status, data.status_text, data.time.as_millis());
                    let _ = cx.update(|cx| {
                        cx.update_global::<super::history::RequestHistory, _>(|history, _| {
                            history.update_response(history_id, data.status, data.time);
                        });

                        if !data.extracted_vars.is_empty() || !data.env_changes.is_empty() {
                            if let Some(explorer) = &explorer_panel {
                                explorer.update(cx, |panel, cx| {
                                    for (name, value) in
                                        data.extracted_vars.iter().chain(data.env_changes.iter())
                                    {
                                        panel.set_env_variable(name, value, cx);
                                    }
                                });
                            }
                        }

                        if let Some(console) = &console_panel {
                            let duration_ms = data.time.as_millis() as u64;
                            let status = data.status;
                            let body_preview = data.body.clone();
                            console.update(cx, |panel, cx| {
                                panel.log(ConsoleEntry {
                                    timestamp: chrono::Local::now(),
                                    level: LogLevel::Info,
                                    source: ConsoleEntrySource::Request,
                                    protocol: log_protocol.to_string(),
                                    method: log_method.clone(),
                                    url: log_url.clone(),
                                    status,
                                    duration_ms,
                                    error: None,
                                    response_body: body_preview,
                                    troubleshoot_hint: None,
                                }, cx);
                            });
                        }

                        response_panel.update(cx, |panel, cx| {
                            panel.set_response(
                                ResponseData {
                                    status: data.status,
                                    status_text: data.status_text,
                                    headers: data.headers,
                                    body: data.body,
                                    time: data.time,
                                    size: data.size,
                                },
                                cx,
                            );
                            if !data.test_results.is_empty() {
                                panel.set_test_results(data.test_results, cx);
                            }
                        });
                    });
                }
                Err(e) => {
                    error!("[{}] Request failed {}: {}", log_protocol, log_url, e);
                    let _ = cx.update(|cx| {
                        if let Some(console) = &console_panel {
                            let err = e.clone();
                            let hint = dns_troubleshoot_hint(&err);
                            console.update(cx, |panel, cx| {
                                panel.log(ConsoleEntry {
                                    timestamp: chrono::Local::now(),
                                    level: LogLevel::Error,
                                    source: ConsoleEntrySource::Request,
                                    protocol: log_protocol.to_string(),
                                    method: log_method.clone(),
                                    url: log_url.clone(),
                                    status: 0,
                                    duration_ms: 0,
                                    error: Some(err),
                                    response_body: String::new(),
                                    troubleshoot_hint: hint,
                                }, cx);
                            });
                        }
                        response_panel.update(cx, |panel, cx| {
                            panel.set_error(e, cx);
                        });
                    });
                }
            }

            let _ = cx.update(|cx| {
                let _ = this.update(cx, |this, cx| {
                    this.loading = false;
                    cx.notify();
                });
            });
        })
        .detach();
    }

    fn focus_url(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.url_focus.focus(window, cx);
    }

    fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        let char_count = self.url.chars().count();
        let offset = offset.min(char_count);
        self.url_selection = offset..offset;
        cx.notify();
    }

    fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        let char_count = self.url.chars().count();
        let offset = offset.min(char_count);
        self.url_selection.end = offset;
        // Normalize range
        if self.url_selection.end < self.url_selection.start {
            self.url_selection = self.url_selection.end..self.url_selection.start;
        }
        cx.notify();
    }

    fn select_all(&mut self, cx: &mut Context<Self>) {
        self.url_selection = 0..self.url.chars().count();
        cx.notify();
    }

    fn has_selection(&self) -> bool {
        self.url_selection.start != self.url_selection.end
    }

    fn selected_text(&self) -> String {
        let byte_start = char_to_byte_offset(&self.url, self.url_selection.start);
        let byte_end = char_to_byte_offset(&self.url, self.url_selection.end);
        self.url[byte_start..byte_end].to_string()
    }

    fn delete_selection(&mut self, cx: &mut Context<Self>) {
        if self.has_selection() {
            self.save_url_state();
            self.delete_selection_no_save(cx);
        }
    }

    /// Delete selection without saving to undo (used internally)
    fn delete_selection_no_save(&mut self, cx: &mut Context<Self>) {
        if self.has_selection() {
            let char_start = self.url_selection.start.min(self.url_selection.end);
            let char_end = self.url_selection.start.max(self.url_selection.end);
            // Convert character indices to byte offsets
            let byte_start = char_to_byte_offset(&self.url, char_start);
            let byte_end = char_to_byte_offset(&self.url, char_end);
            self.url.replace_range(byte_start..byte_end, "");
            self.url_selection = char_start..char_start;
            self.sync_params_from_url(cx);
            cx.notify();
        }
    }

    /// Save URL state to undo stack before making changes
    fn save_url_state(&mut self) {
        self.url_undo_stack.push((self.url.clone(), self.url_selection.clone()));
        if self.url_undo_stack.len() > 100 {
            self.url_undo_stack.remove(0);
        }
        self.url_redo_stack.clear();
    }

    /// Undo URL change
    fn url_undo(&mut self, cx: &mut Context<Self>) {
        if let Some((text, selection)) = self.url_undo_stack.pop() {
            self.url_redo_stack.push((self.url.clone(), self.url_selection.clone()));
            self.url = text;
            self.url_selection = selection;
            self.sync_params_from_url(cx);
            cx.notify();
        }
    }

    /// Redo URL change
    fn url_redo(&mut self, cx: &mut Context<Self>) {
        if let Some((text, selection)) = self.url_redo_stack.pop() {
            self.url_undo_stack.push((self.url.clone(), self.url_selection.clone()));
            self.url = text;
            self.url_selection = selection;
            self.sync_params_from_url(cx);
            cx.notify();
        }
    }

    fn insert_text(&mut self, text: &str, cx: &mut Context<Self>) {
        self.save_url_state();
        self.delete_selection_no_save(cx);
        let char_pos = self.url_selection.start;
        // Convert character index to byte offset for string operation
        let byte_pos = char_to_byte_offset(&self.url, char_pos);
        self.url.insert_str(byte_pos, text);
        // New position is after the inserted text (in character indices)
        let new_char_pos = char_pos + text.chars().count();
        self.url_selection = new_char_pos..new_char_pos;
        self.sync_params_from_url(cx);
        cx.notify();
    }

    fn index_for_x(&self, x: f32) -> usize {
        // Approximate character position from x coordinate
        // ~7.8px per character at 13px font size
        let char_width: f32 = 7.8;
        if x <= 0.0 {
            0
        } else {
            let approx_char = (x / char_width) as usize;
            approx_char.min(self.url.chars().count())
        }
    }

    fn handle_url_mouse_down(&mut self, event: &MouseDownEvent, cx: &mut Context<Self>) {
        self.is_selecting = true;
        // url_input_left is set by canvas to the text content start in window coords
        let click_x = (f32::from(event.position.x) - self.url_input_left).max(0.0);
        let index = self.index_for_x(click_x);

        // Cycle: 1=cursor, 2=word, 3=all, 4+=cursor
        let effective_click = if event.click_count >= 4 { 1 } else { event.click_count };

        match effective_click {
            2 => {
                // Double-click: select word
                let start = find_word_start(&self.url, index);
                let end = find_word_end(&self.url, index);
                self.url_selection = start..end;
                cx.notify();
            }
            3 => {
                // Triple-click: select all
                self.select_all(cx);
            }
            _ => {
                // Single click (or 4th+ click to deselect)
                if event.modifiers.shift {
                    self.select_to(index, cx);
                } else {
                    self.move_to(index, cx);
                }
            }
        }
    }

    fn handle_url_mouse_move(&mut self, event: &MouseMoveEvent, cx: &mut Context<Self>) {
        if self.is_selecting {
            let click_x = (f32::from(event.position.x) - self.url_input_left).max(0.0);
            let index = self.index_for_x(click_x);
            self.url_selection.end = index.min(self.url.chars().count());
            cx.notify();
        }
    }

    fn handle_url_mouse_up(&mut self, _event: &MouseUpEvent, _cx: &mut Context<Self>) {
        self.is_selecting = false;
    }

    fn handle_url_key(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let key = event.keystroke.key.as_str();
        let ctrl = event.keystroke.modifiers.control;
        let shift = event.keystroke.modifiers.shift;

        // Handle Ctrl shortcuts
        if ctrl {
            match key {
                "enter" => {
                    // Ctrl+Enter = Send request
                    self.send_request(cx);
                    return;
                }
                "a" => {
                    self.select_all(cx);
                    return;
                }
                "c" => {
                    if self.has_selection() {
                        cx.write_to_clipboard(ClipboardItem::new_string(
                            self.selected_text().to_string(),
                        ));
                    }
                    return;
                }
                "x" => {
                    if self.has_selection() {
                        cx.write_to_clipboard(ClipboardItem::new_string(
                            self.selected_text().to_string(),
                        ));
                        self.delete_selection(cx);
                    }
                    return;
                }
                "v" => {
                    if let Some(item) = cx.read_from_clipboard() {
                        if let Some(text) = item.text() {
                            self.insert_text(&text.replace('\n', ""), cx);
                        }
                    }
                    return;
                }
                "z" => {
                    if shift {
                        // Ctrl+Shift+Z = Redo
                        self.url_redo(cx);
                    } else {
                        // Ctrl+Z = Undo
                        self.url_undo(cx);
                    }
                    return;
                }
                "y" => {
                    // Ctrl+Y = Redo (alternative)
                    self.url_redo(cx);
                    return;
                }
                _ => {}
            }
        }

        match key {
            "left" => {
                if shift {
                    if self.url_selection.end > 0 {
                        self.url_selection.end -= 1;
                        cx.notify();
                    }
                } else if self.has_selection() {
                    let start = self.url_selection.start.min(self.url_selection.end);
                    self.move_to(start, cx);
                } else if self.cursor() > 0 {
                    self.move_to(self.cursor() - 1, cx);
                }
            }
            "right" => {
                let char_count = self.url.chars().count();
                if shift {
                    if self.url_selection.end < char_count {
                        self.url_selection.end += 1;
                        cx.notify();
                    }
                } else if self.has_selection() {
                    let end = self.url_selection.start.max(self.url_selection.end);
                    self.move_to(end, cx);
                } else if self.cursor() < char_count {
                    self.move_to(self.cursor() + 1, cx);
                }
            }
            "home" => {
                if shift {
                    self.url_selection.end = 0;
                    cx.notify();
                } else {
                    self.move_to(0, cx);
                }
            }
            "end" => {
                let char_count = self.url.chars().count();
                if shift {
                    self.url_selection.end = char_count;
                    cx.notify();
                } else {
                    self.move_to(char_count, cx);
                }
            }
            "backspace" => {
                if self.has_selection() {
                    self.delete_selection(cx);
                } else if self.cursor() > 0 {
                    self.save_url_state();
                    let char_pos = self.cursor() - 1;
                    // Convert char position to byte offset and remove that char
                    let byte_pos = char_to_byte_offset(&self.url, char_pos);
                    let next_byte_pos = char_to_byte_offset(&self.url, char_pos + 1);
                    self.url.replace_range(byte_pos..next_byte_pos, "");
                    self.url_selection = char_pos..char_pos;
                    self.sync_params_from_url(cx);
                    cx.notify();
                }
            }
            "delete" => {
                let char_count = self.url.chars().count();
                if self.has_selection() {
                    self.delete_selection(cx);
                } else if self.cursor() < char_count {
                    self.save_url_state();
                    let cursor = self.cursor();
                    // Convert char position to byte offset and remove that char
                    let byte_pos = char_to_byte_offset(&self.url, cursor);
                    let next_byte_pos = char_to_byte_offset(&self.url, cursor + 1);
                    self.url.replace_range(byte_pos..next_byte_pos, "");
                    self.sync_params_from_url(cx);
                    cx.notify();
                }
            }
            "enter" => {
                self.send_request(cx);
            }
            _ => {
                // Handle printable characters
                if let Some(ch) = &event.keystroke.key_char {
                    self.insert_text(ch, cx);
                }
            }
        }
        self.update_url_scroll();
    }

    pub(super) fn update_url_scroll(&mut self) {
        let char_width = 13.0 * 0.6; // font_size 13 * 0.6 monospace ratio
        let padding = 14.0 * 2.0;    // px(14) each side
        let visible_width = (self.url_input_width - padding).max(60.0);
        let cursor_px = self.url_selection.end as f32 * char_width;

        if cursor_px < self.url_scroll_offset {
            self.url_scroll_offset = cursor_px;
        } else if cursor_px > self.url_scroll_offset + visible_width - char_width {
            self.url_scroll_offset = cursor_px - visible_width + char_width;
        }
        if self.url_scroll_offset < 0.0 {
            self.url_scroll_offset = 0.0;
        }
    }
}

impl<E: WebSocketExecutor> Render for RequestPanel<E> {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Reset skip_blur flag at start of each render
        self.skip_blur = false;

        div()
            .id("request-panel")
            .size_full()
            .flex()
            .flex_col()
            .relative()
            .track_focus(&self.body_focus)
            .capture_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                // Ctrl+S = Save
                if event.keystroke.modifiers.control && event.keystroke.key == "s" {
                    this.save_request(cx);
                    return;
                }

                // Close dropdowns on Escape key
                if event.keystroke.key == "escape" {
                    if this.mode_dropdown_open {
                        this.mode_dropdown_open = false;
                        cx.notify();
                        return;
                    }
                    if this.method_dropdown_open {
                        this.method_dropdown_open = false;
                        cx.notify();
                        return;
                    }
                }

                // Route key events based on active_edit
                if this.active_edit.is_some() {
                    this.handle_edit_key(event, cx);
                }
            }))
            .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
                // Only clear focus if an input wasn't clicked
                if !this.skip_blur && this.active_edit.is_some() {
                    this.active_edit = None;
                    cx.notify();
                }
                // Close dropdowns when clicking outside
                if this.method_dropdown_open {
                    this.method_dropdown_open = false;
                    cx.notify();
                }
                if this.mode_dropdown_open {
                    this.mode_dropdown_open = false;
                    cx.notify();
                }
            }))
            // URL bar
            .child(self.render_url_bar(window, cx))
            // Tabs
            .child(self.render_tabs(cx))
            // Tab content
            .child(
                div()
                    .id("tab-content")
                    .flex_1()
                    .w_full()
                    .p(px(12.0))
                    .overflow_scroll()
                    .child(self.render_tab_content(cx)),
            )
            // Floating dropdown overlays — wrapped in deferred() to paint above overflow_scroll
            .when(self.method_dropdown_open, |el| {
                el.child(deferred(self.render_method_dropdown_overlay(window, cx)).with_priority(1))
            })
            .when(self.mode_dropdown_open, |el| {
                el.child(deferred(self.render_mode_dropdown_overlay(cx)).with_priority(1))
            })
            // KV column resize overlay
            .when(self.kv_col_drag.is_some(), |el| {
                el.child(deferred(
                    div()
                        .id("kv-col-resize-overlay")
                        .absolute().top_0().left_0().w_full().h_full()
                        .cursor_col_resize()
                        .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                            if let Some((start_x, start_w)) = this.kv_col_drag {
                                let delta = f32::from(event.position.x) - start_x;
                                let new_w = (start_w + delta).max(60.0).min(500.0);
                                if (this.kv_col_key_w - new_w).abs() > 0.5 {
                                    this.kv_col_key_w = new_w;
                                    cx.notify();
                                }
                            }
                        }))
                        .on_mouse_up(MouseButton::Left, cx.listener(|this, _, _, cx| {
                            this.kv_col_drag = None;
                            cx.notify();
                        }))
                ).with_priority(1))
            })
    }

}

// ── GraphQL schema helpers ────────────────────────────────────────────────────

/// Send the introspection query to `url` and return a `GraphqlSchemaState`.
/// Runs on the background executor (blocking reqwest).
fn run_graphql_introspection(url: &str) -> GraphqlSchemaState {
    const INTROSPECTION: &str = r#"{"query":"{__schema{types{name kind description}}}"}"#;

    let client = match reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
    {
        Ok(c) => c,
        Err(e) => return GraphqlSchemaState::Error(e.to_string()),
    };

    let resp = client
        .post(url)
        .header("Content-Type", "application/json")
        .body(INTROSPECTION)
        .send();

    match resp {
        Err(e) => GraphqlSchemaState::Error(e.to_string()),
        Ok(r) => match r.json::<serde_json::Value>() {
            Err(e) => GraphqlSchemaState::Error(format!("Parse error: {e}")),
            Ok(json) => extract_schema_types(&json),
        },
    }
}

fn extract_schema_types(json: &serde_json::Value) -> GraphqlSchemaState {
    let types = json
        .pointer("/data/__schema/types")
        .and_then(|v| v.as_array());

    match types {
        None => GraphqlSchemaState::Error("Unexpected introspection response shape".into()),
        Some(arr) => {
            let types: Vec<GqlSchemaType> = arr
                .iter()
                .filter_map(|t| {
                    let name = t.get("name")?.as_str()?.to_string();
                    // Skip built-in introspection types
                    if name.starts_with("__") {
                        return None;
                    }
                    Some(GqlSchemaType {
                        name,
                        kind: t.get("kind").and_then(|k| k.as_str()).unwrap_or("").to_string(),
                        description: t.get("description").and_then(|d| d.as_str()).map(|s| s.to_string()),
                    })
                })
                .collect();
            GraphqlSchemaState::Loaded(types)
        }
    }
}

/// Parse a local .graphql/.gql or .json file into a `GraphqlSchemaState`.
fn parse_schema_file(path: &std::path::Path) -> GraphqlSchemaState {
    let content = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => return GraphqlSchemaState::Error(e.to_string()),
    };

    // Try JSON introspection result first
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
        return extract_schema_types(&json);
    }

    // For .graphql SDL files, extract type definitions by name
    let types: Vec<GqlSchemaType> = content
        .lines()
        .filter_map(|line| {
            let t = line.trim();
            for prefix in &["type ", "interface ", "enum ", "union ", "input ", "scalar "] {
                if t.starts_with(prefix) {
                    let rest = t[prefix.len()..].split_whitespace().next()?;
                    let name = rest.trim_end_matches('{').to_string();
                    if !name.starts_with("__") {
                        return Some(GqlSchemaType {
                            name,
                            kind: prefix.trim().to_uppercase(),
                            description: None,
                        });
                    }
                }
            }
            None
        })
        .collect();

    if types.is_empty() {
        GraphqlSchemaState::Error("No type definitions found in file".into())
    } else {
        GraphqlSchemaState::Loaded(types)
    }
}

/// Return an actionable troubleshooting hint when `err` looks like a DNS or
/// network-reachability failure, or `None` for ordinary HTTP errors.
fn dns_troubleshoot_hint(err: &str) -> Option<String> {
    let lower = err.to_lowercase();
    if lower.contains("resolve")
        || lower.contains("no such host")
        || lower.contains("dns")
        || lower.contains("name or service not known")
        || lower.contains("nodename nor servname")
        || lower.contains("unable to resolve")
        || lower.contains("failed to lookup")
        || lower.contains("connection refused")
        || lower.contains("network unreachable")
        || lower.contains("timed out")
    {
        Some(
            "1) Verify the hostname spelling in the URL.\n\
             2) Run: nslookup <hostname>  (or dig <hostname>)\n\
             3) Test basic connectivity: ping 8.8.8.8\n\
             4) For private/local hosts, check /etc/hosts or your VPN config.\n\
             5) If using a custom port, confirm the service is running: nc -zv <host> <port>"
                .to_string(),
        )
    } else {
        None
    }
}


