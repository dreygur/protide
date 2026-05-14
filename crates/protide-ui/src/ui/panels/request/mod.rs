//! Request editor panel – URL input, method selector, headers/params/body, auth.

mod render;
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
mod render_trpc;
mod render_socketio;
mod render_socketio_helpers;

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
mod execution_http;
mod execution_ws;
mod execution_sio;
mod execution_grpc;
mod execution_trpc;

#[cfg(test)]
mod tests;

use std::ops::Range;
use std::marker::PhantomData;
use gpui::{
    deferred, div, prelude::*, px, ClipboardItem, Context, Entity, FocusHandle, IntoElement,
    KeyDownEvent, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, Render,
    Styled, Subscription, Window,
};
use crate::ui::components::{render_text_view_with_max, find_word_start, find_word_end};
use crate::ui::components::code_editor::{CodeEditor, Language};
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
    GrpcMethodInfo, GrpcStreamingType, HttpMethod, KeyValuePair, RequestMode,
    SioConnectionState, WsConnectionState,
};
use super::request_utils::{base64_encode, status_text, url_decode, url_encode};
use base64::Engine;
use super::response::{ResponseData, ResponsePanel};
use protide_core::codegen::{self as protide_codegen, CodegenRequest, Language as CodegenLanguage};
use protide_core::import as protide_import;
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

fn render_text_view(
    text: &str, selection: &Range<usize>, is_focused: bool, font_size: f32,
    text_color: gpui::Hsla, placeholder: Option<&str>, placeholder_color: gpui::Hsla,
    selection_bg: gpui::Hsla,
) -> gpui::AnyElement {
    render_text_view_with_max(text, selection, is_focused, font_size, text_color, placeholder, placeholder_color, None, selection_bg)
}

fn char_to_byte_offset(text: &str, char_idx: usize) -> usize {
    text.char_indices().nth(char_idx).map(|(b, _)| b).unwrap_or(text.len())
}

#[allow(dead_code)]
fn byte_to_char_offset(text: &str, byte_offset: usize) -> usize {
    text[..byte_offset.min(text.len())].chars().count()
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
    pub(super) url_undo_stack: Vec<(String, Range<usize>)>,
    pub(super) url_redo_stack: Vec<(String, Range<usize>)>,
    pub(super) edit_undo_stack: Vec<(EditTarget, String, Range<usize>)>,
    pub(super) edit_redo_stack: Vec<(EditTarget, String, Range<usize>)>,
    pub(super) skip_blur: bool,
    pub(super) edit_focus: FocusHandle,
    pub(super) body_focus: FocusHandle,
    pub(super) explorer_panel: Option<Entity<ExplorerPanel>>,
    pub(super) body_editor: Entity<CodeEditor>,
    pub(super) pre_script: String,
    pub(super) post_script: String,
    pub(super) tests: String,
    pub(super) pre_script_editor: Entity<CodeEditor>,
    pub(super) post_script_editor: Entity<CodeEditor>,
    pub(super) tests_editor: Entity<CodeEditor>,
    pub(super) variable_extractions: Vec<VariableExtraction>,
    pub codegen_content: Option<String>,
    pub codegen_language: CodegenLanguage,
    pub codegen_editor: Entity<CodeEditor>,
    pub import_modal_open: bool,
    pub(super) import_text: String,
    pub(super) import_error: Option<String>,
    pub(super) import_editor: Entity<CodeEditor>,
    pub(super) request_mode: RequestMode,
    pub(super) graphql_query_editor: Entity<CodeEditor>,
    pub(super) graphql_variables_editor: Entity<CodeEditor>,
    pub(super) graphql_operation_name: String,
    pub(super) ws_state: WsConnectionState,
    pub(super) ws_messages: WsRingBuffer,
    pub(super) ws_message_input: String,
    pub(super) ws_message_editor: Entity<CodeEditor>,
    ws_send_tx: Option<std::sync::mpsc::Sender<WsCommand>>,
    pub(super) ws_compose_h: f32,
    pub(super) ws_compose_drag: Option<(f32, f32)>,
    pub(super) grpc_message_editor: Entity<CodeEditor>,
    pub(super) grpc_metadata: Vec<KeyValuePair>,
    pub(super) grpc_proto_path: Option<std::path::PathBuf>,
    pub(super) grpc_proto_content: String,
    pub(super) grpc_services: Vec<String>,
    pub(super) grpc_service: Option<String>,
    pub(super) grpc_methods: Vec<GrpcMethodInfo>,
    pub(super) grpc_method: Option<GrpcMethodInfo>,
    pub(super) trpc_procedure: String,
    pub(super) trpc_params_editor: Entity<CodeEditor>,
    pub(super) sio_state: SioConnectionState,
    pub(super) sio_messages: SioRingBuffer,
    pub(super) sio_namespace: String,
    pub(super) sio_event_name: String,
    pub(super) sio_want_ack: bool,
    pub(super) sio_next_ack_id: u32,
    pub(super) sio_payload_editor: Entity<CodeEditor>,
    sio_send_tx: Option<std::sync::mpsc::Sender<SioCommand>>,
    pub(super) kv_col_key_w: f32,
    pub(super) kv_col_drag: Option<(f32, f32)>,
    pub(super) script_pre_open: bool,
    pub(super) script_post_open: bool,
    pub(super) script_tests_open: bool,
    pub(super) script_pre_h: f32,
    pub(super) script_post_h: f32,
    pub(super) drag_script_pre: Option<(f32, f32)>,
    pub(super) drag_script_post: Option<(f32, f32)>,
    pub(super) current_file: Option<std::path::PathBuf>,
    pub(super) save_feedback: bool,
    pub(super) custom_method_input: String,
    pub(super) custom_method_focus: FocusHandle,
    pub(super) graphql_schema: GraphqlSchemaState,
    pub(super) graphql_schema_search: String,
    pub(super) console_panel: Option<Entity<ConsolePanel>>,
    _executor: PhantomData<E>,
}

impl<E: WebSocketExecutor> Render for RequestPanel<E> {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.skip_blur = false;
        div()
            .id("request-panel")
            .size_full().flex().flex_col().relative()
            .track_focus(&self.body_focus)
            .capture_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                if event.keystroke.modifiers.control && event.keystroke.key == "s" {
                    this.save_request(cx); return;
                }
                if event.keystroke.key == "escape" {
                    if this.mode_dropdown_open { this.mode_dropdown_open = false; cx.notify(); return; }
                    if this.method_dropdown_open { this.method_dropdown_open = false; cx.notify(); return; }
                }
                if this.active_edit.is_some() { this.handle_edit_key(event, cx); }
            }))
            .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
                if !this.skip_blur && this.active_edit.is_some() { this.active_edit = None; cx.notify(); }
                if this.method_dropdown_open { this.method_dropdown_open = false; cx.notify(); }
                if this.mode_dropdown_open { this.mode_dropdown_open = false; cx.notify(); }
            }))
            .child(self.render_url_bar(window, cx))
            .child(self.render_tabs(cx))
            .child(div().id("tab-content").flex_1().w_full().p(px(12.0)).overflow_scroll()
                .child(self.render_tab_content(cx)))
            .when(self.method_dropdown_open, |el|
                el.child(deferred(self.render_method_dropdown_overlay(window, cx)).with_priority(1)))
            .when(self.mode_dropdown_open, |el|
                el.child(deferred(self.render_mode_dropdown_overlay(cx)).with_priority(1)))
            .when(self.kv_col_drag.is_some(), |el| el.child(deferred(
                div().id("kv-col-resize-overlay")
                    .absolute().top_0().left_0().w_full().h_full()
                    .cursor_col_resize()
                    .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                        if let Some((start_x, start_w)) = this.kv_col_drag {
                            let new_w = (start_w + f32::from(event.position.x) - start_x).max(60.0).min(500.0);
                            if (this.kv_col_key_w - new_w).abs() > 0.5 { this.kv_col_key_w = new_w; cx.notify(); }
                        }
                    }))
                    .on_mouse_up(MouseButton::Left, cx.listener(|this, _, _, cx| {
                        this.kv_col_drag = None; cx.notify();
                    }))
            ).with_priority(1)))
    }
}
