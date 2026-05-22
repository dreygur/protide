//! Request panel types and enums
//!
//! Contains shared types used by the request panel.

use std::path::PathBuf;

/// Key-value pair for headers, params, etc.
#[derive(Clone, Default)]
pub struct KeyValuePair {
    pub key: String,
    pub value: String,
    pub enabled: bool,
}

/// Identifies which request-panel editor a deferred content update targets.
/// Used to apply `InputState::set_value` (which needs `&mut Window`) during render.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PendingEditor {
    Body,
    PreScript,
    PostScript,
    Tests,
    GraphqlQuery,
    GraphqlVariables,
    GrpcMessage,
    TrpcParams,
    SioPayload,
}

/// Form field type (text or file)
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum FormFieldType {
    #[default]
    Text,
    File,
}

/// Form field that can be text or file
#[derive(Clone, Default)]
pub struct FormField {
    pub key: String,
    pub value: String,           // Text value or display name for file
    pub field_type: FormFieldType,
    pub file_path: Option<PathBuf>, // Path to file when field_type is File
    pub enabled: bool,
}

/// Target for text editing
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EditTarget {
    ParamKey(usize),
    ParamValue(usize),
    HeaderKey(usize),
    HeaderValue(usize),
    BearerToken,
    BasicUsername,
    BasicPassword,
    ApiKeyName,
    ApiKeyValue,
    FormKey(usize),
    FormValue(usize),
    GrpcMetaKey(usize),
    GrpcMetaValue(usize),
    TrpcProcedure,
    SioNamespace,
    SioEventName,
    SioRoomName,
}

/// Authentication type
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AuthType {
    #[default]
    None,
    Bearer,
    Basic,
    ApiKey,
}

/// API key location
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ApiKeyLocation {
    #[default]
    Header,
    QueryParam,
}

/// HTTP request method
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum HttpMethod {
    #[default]
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Custom(String),
}

impl HttpMethod {
    pub fn as_str(&self) -> &str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Delete => "DELETE",
            HttpMethod::Custom(s) => s.as_str(),
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        let upper = s.to_uppercase();
        match upper.as_str() {
            "GET" => Some(HttpMethod::Get),
            "POST" => Some(HttpMethod::Post),
            "PUT" => Some(HttpMethod::Put),
            "PATCH" => Some(HttpMethod::Patch),
            "DELETE" => Some(HttpMethod::Delete),
            "" => None,
            _ => Some(HttpMethod::Custom(upper)),
        }
    }

    pub fn all() -> &'static [HttpMethod] {
        &[
            HttpMethod::Get,
            HttpMethod::Post,
            HttpMethod::Put,
            HttpMethod::Patch,
            HttpMethod::Delete,
        ]
    }
}

/// Body content type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BodyType {
    #[default]
    Json,
    Xml,
    Raw,
    Form,
    Binary,
}

/// Request mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RequestMode {
    #[default]
    Http,
    GraphQL,
    WebSocket,
    Grpc,
    Trpc,
    SocketIo,
}

/// gRPC streaming type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GrpcStreamingType {
    #[default]
    Unary,
    ServerStreaming,
    ClientStreaming,
    BidiStreaming,
}

impl GrpcStreamingType {
    pub fn label(&self) -> &'static str {
        match self {
            GrpcStreamingType::Unary => "unary",
            GrpcStreamingType::ServerStreaming => "server",
            GrpcStreamingType::ClientStreaming => "client",
            GrpcStreamingType::BidiStreaming => "bidi",
        }
    }
}

/// gRPC method info with streaming type
#[derive(Debug, Clone)]
pub struct GrpcMethodInfo {
    pub full_name: String,
    pub streaming_type: GrpcStreamingType,
}

/// WebSocket connection state (UI-only: tracks transitional Connecting phase)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WsConnectionState {
    #[default]
    Disconnected,
    Connecting,
    Connected,
    Error,
}

/// Socket.IO connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SioConnectionState {
    #[default]
    Disconnected,
    Connecting,
    Connected,
}

/// Result row from a data-driven CSV test run
#[derive(Debug, Clone)]
pub struct DataRunRow {
    pub row: usize,
    pub status: Option<u16>,
    pub passed: bool,
    pub error: Option<String>,
}

/// Identifies which KV list is being row-dragged
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KvList {
    Params,
    Headers,
    GrpcMeta,
}

