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
        self.body.as_ref().is_some_and(|b| !b.trim().is_empty())
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Every variant, so a newly added method can't silently miss `from_str`.
    const ALL_METHODS: &[HttpMethod] = &[
        HttpMethod::Get,
        HttpMethod::Post,
        HttpMethod::Put,
        HttpMethod::Patch,
        HttpMethod::Delete,
        HttpMethod::Head,
        HttpMethod::Options,
        HttpMethod::Connect,
        HttpMethod::Trace,
        HttpMethod::WebSocket,
        HttpMethod::Grpc,
    ];

    #[test]
    fn method_round_trips_through_as_str() {
        for m in ALL_METHODS {
            assert_eq!(
                HttpMethod::from_str(m.as_str()),
                Some(*m),
                "{} must parse back to itself",
                m.as_str()
            );
        }
    }

    #[test]
    fn method_parsing_is_case_insensitive() {
        assert_eq!(HttpMethod::from_str("get"), Some(HttpMethod::Get));
        assert_eq!(HttpMethod::from_str("GeT"), Some(HttpMethod::Get));
        assert_eq!(HttpMethod::from_str("dElEtE"), Some(HttpMethod::Delete));
        assert_eq!(HttpMethod::from_str("ws"), Some(HttpMethod::WebSocket));
        assert_eq!(HttpMethod::from_str("Ws"), Some(HttpMethod::WebSocket));
        assert_eq!(HttpMethod::from_str("grpc"), Some(HttpMethod::Grpc));
    }

    #[test]
    fn method_parsing_rejects_junk_without_panicking() {
        // Empty, whitespace-padded, near-misses, non-ASCII and multi-byte
        // input all take the `to_uppercase()` path; none may panic.
        for junk in [
            "",
            " ",
            "\t",
            " GET",
            "GET ",
            "G",
            "GETS",
            "GE T",
            "получить",
            "🚀",
            "İ",
            "ß",
            "GET\u{0}",
            "\u{feff}GET",
        ] {
            assert_eq!(
                HttpMethod::from_str(junk),
                None,
                "{junk:?} must not parse as a method"
            );
        }
    }

    #[test]
    fn method_default_is_get() {
        assert_eq!(HttpMethod::default(), HttpMethod::Get);
        assert_eq!(HttpMethod::default().as_str(), "GET");
    }

    #[test]
    fn key_value_new_is_enabled_and_accepts_borrowed_or_owned() {
        let kv = KeyValue::new("Accept", String::from("*/*"));
        assert_eq!(kv.key, "Accept");
        assert_eq!(kv.value, "*/*");
        assert!(kv.enabled, "a freshly built pair is enabled");
    }

    #[test]
    fn request_new_starts_empty() {
        let r = Request::new(HttpMethod::Post, "https://example.test".to_string());
        assert_eq!(r.method, HttpMethod::Post);
        assert_eq!(r.url, "https://example.test");
        assert!(r.headers.is_empty());
        assert!(r.body.is_none());
        assert!(!r.has_body());
        assert_eq!(r.line, 0);
        assert!(r.meta.name.is_none());
        assert!(r.meta.variable_extractions.is_empty());
        assert!(r.scripts.pre_script.is_none());
    }

    #[test]
    fn protocol_is_inferred_from_method_unless_overridden() {
        let infer = |m| Request::new(m, String::new()).protocol();
        assert_eq!(infer(HttpMethod::Get), Protocol::Http);
        assert_eq!(infer(HttpMethod::Delete), Protocol::Http);
        assert_eq!(infer(HttpMethod::WebSocket), Protocol::WebSocket);
        assert_eq!(infer(HttpMethod::Grpc), Protocol::Grpc);

        // An explicit @protocol always wins, even when it contradicts the method.
        let mut r = Request::new(HttpMethod::Grpc, String::new());
        r.meta.protocol = Some(Protocol::Trpc);
        assert_eq!(r.protocol(), Protocol::Trpc);
    }

    #[test]
    fn has_body_ignores_whitespace_only_bodies() {
        let with = |b: Option<&str>| {
            let mut r = Request::new(HttpMethod::Post, String::new());
            r.body = b.map(str::to_string);
            r.has_body()
        };
        assert!(!with(None));
        assert!(!with(Some("")));
        assert!(!with(Some("   \n\t\r\n  ")));
        assert!(with(Some("{}")));
        assert!(with(Some("  🚀  ")));
    }

    #[test]
    fn get_header_matches_case_insensitively_and_skips_disabled() {
        let mut r = Request::new(HttpMethod::Get, String::new());
        r.headers = vec![
            KeyValue::new("Content-Type", "application/json"),
            KeyValue {
                key: "X-Off".to_string(),
                value: "hidden".to_string(),
                enabled: false,
            },
        ];

        assert_eq!(r.get_header("content-TYPE"), Some("application/json"));
        assert_eq!(r.get_header("Content-Type"), Some("application/json"));
        assert_eq!(
            r.get_header("X-Off"),
            None,
            "disabled headers are invisible"
        );
        assert_eq!(r.get_header("X-Missing"), None);
        assert_eq!(r.get_header(""), None);
    }

    #[test]
    fn get_header_returns_the_first_enabled_duplicate() {
        let mut r = Request::new(HttpMethod::Get, String::new());
        r.headers = vec![
            KeyValue::new("Accept", "first"),
            KeyValue::new("accept", "second"),
        ];
        assert_eq!(r.get_header("Accept"), Some("first"));
    }

    #[test]
    fn http_file_default_matches_new() {
        let f = HttpFile::default();
        assert!(f.requests.is_empty());
        assert!(f.variables.is_empty());
        assert_eq!(HttpFile::new().requests.len(), f.requests.len());
    }
}
