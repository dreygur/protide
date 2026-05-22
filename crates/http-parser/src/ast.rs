//! AST types for parsed .http files

use serde::{Deserialize, Serialize};

/// Protocol type for the request
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    #[default]
    Http,
    GraphQL,
    WebSocket,
    Grpc,
    SocketIO,
    Trpc,
}

/// HTTP method
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    #[default]
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
    Connect,
    Trace,
    // Extended methods for other protocols
    #[serde(rename = "WEBSOCKET")]
    WebSocket,
    #[serde(rename = "GRPC")]
    Grpc,
}

impl HttpMethod {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "GET" => Some(Self::Get),
            "POST" => Some(Self::Post),
            "PUT" => Some(Self::Put),
            "PATCH" => Some(Self::Patch),
            "DELETE" => Some(Self::Delete),
            "HEAD" => Some(Self::Head),
            "OPTIONS" => Some(Self::Options),
            "CONNECT" => Some(Self::Connect),
            "TRACE" => Some(Self::Trace),
            "WEBSOCKET" | "WS" => Some(Self::WebSocket),
            "GRPC" => Some(Self::Grpc),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Patch => "PATCH",
            Self::Delete => "DELETE",
            Self::Head => "HEAD",
            Self::Options => "OPTIONS",
            Self::Connect => "CONNECT",
            Self::Trace => "TRACE",
            Self::WebSocket => "WEBSOCKET",
            Self::Grpc => "GRPC",
        }
    }
}

/// A key-value pair (for headers, params, etc.)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyValue {
    pub key: String,
    pub value: String,
    pub enabled: bool,
}

impl KeyValue {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
            enabled: true,
        }
    }
}

/// Request metadata from annotations
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RequestMeta {
    /// Request name (@name)
    pub name: Option<String>,
    /// Description (@description)
    pub description: Option<String>,
    /// Protocol override (@protocol)
    pub protocol: Option<Protocol>,
    /// Proto file path for gRPC (@proto)
    pub proto_path: Option<String>,
    /// Dependencies on other requests (@depends)
    pub depends: Vec<String>,
    /// Variables to set from response (@set)
    pub variable_extractions: Vec<VariableExtraction>,
}

/// Variable extraction from response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableExtraction {
    /// Variable name to set
    pub name: String,
    /// JSONPath or XPath expression
    pub expression: String,
}

/// Scripts attached to a request
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Scripts {
    /// Pre-request script (JavaScript)
    pub pre_script: Option<String>,
    /// Post-response script (JavaScript)
    pub post_script: Option<String>,
    /// Test assertions (JavaScript)
    pub tests: Option<String>,
}

/// A parsed HTTP request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    /// Request metadata from annotations
    pub meta: RequestMeta,
    /// HTTP method
    pub method: HttpMethod,
    /// Request URL (may contain variables like {{base_url}})
    pub url: String,
    /// HTTP headers
    pub headers: Vec<KeyValue>,
    /// Request body (if any)
    pub body: Option<String>,
    /// Scripts (pre, post, tests)
    pub scripts: Scripts,
    /// Line number where request starts (for error reporting)
    pub line: usize,
}

impl Request {
    pub fn new(method: HttpMethod, url: String) -> Self {
        Self {
            meta: RequestMeta::default(),
            method,
            url,
            headers: Vec::new(),
            body: None,
            scripts: Scripts::default(),
            line: 0,
        }
    }

    /// Get the effective protocol (from meta or inferred from method)
    pub fn protocol(&self) -> Protocol {
        if let Some(p) = self.meta.protocol {
            return p;
        }
        match self.method {
            HttpMethod::WebSocket => Protocol::WebSocket,
            HttpMethod::Grpc => Protocol::Grpc,
            _ => Protocol::Http,
        }
    }

    /// Check if request has a body
    pub fn has_body(&self) -> bool {
        self.body.as_ref().map_or(false, |b| !b.trim().is_empty())
    }

    /// Get a header value by name (case-insensitive)
    pub fn get_header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|h| h.key.eq_ignore_ascii_case(name) && h.enabled)
            .map(|h| h.value.as_str())
    }
}

/// A parsed .http file containing multiple requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpFile {
    /// List of requests in the file
    pub requests: Vec<Request>,
    /// File-level variables
    pub variables: Vec<KeyValue>,
}

impl HttpFile {
    pub fn new() -> Self {
        Self {
            requests: Vec::new(),
            variables: Vec::new(),
        }
    }
}

impl Default for HttpFile {
    fn default() -> Self {
        Self::new()
    }
}
