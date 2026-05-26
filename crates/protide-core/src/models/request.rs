//! Request and response models

use crate::execution::http::status_text;
use http_parser::{KeyValue, Protocol, Request as ParsedRequest};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// A request tab in the UI
#[derive(Debug, Clone)]
pub struct RequestTab {
    /// Path to the .http file (if saved)
    pub file_path: Option<PathBuf>,
    /// The parsed request
    pub request: ParsedRequest,
    /// Current response (if any)
    pub response: Option<Response>,
    /// Whether the tab has unsaved changes
    pub is_dirty: bool,
    /// Whether a request is currently in progress
    pub is_loading: bool,
}

impl RequestTab {
    pub fn new(request: ParsedRequest) -> Self {
        Self {
            file_path: None,
            request,
            response: None,
            is_dirty: false,
            is_loading: false,
        }
    }

    pub fn from_file(path: PathBuf, request: ParsedRequest) -> Self {
        Self {
            file_path: Some(path),
            request,
            response: None,
            is_dirty: false,
            is_loading: false,
        }
    }

    /// Get the tab title
    pub fn title(&self) -> String {
        if let Some(name) = &self.request.meta.name {
            name.clone()
        } else if let Some(path) = &self.file_path {
            path.file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "Untitled".to_string())
        } else {
            "Untitled".to_string()
        }
    }
}

/// HTTP response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    /// HTTP status code
    pub status: u16,
    /// Status text (e.g., "OK", "Not Found")
    pub status_text: String,
    /// Response headers
    pub headers: Vec<KeyValue>,
    /// Response body
    pub body: String,
    /// Response time
    pub time: Duration,
    /// Response size in bytes
    pub size: usize,
    /// Protocol used
    pub protocol: Protocol,
}

impl Response {
    pub fn new(status: u16) -> Self {
        Self {
            status,
            status_text: status_text(status).to_string(),
            headers: Vec::new(),
            body: String::new(),
            time: Duration::ZERO,
            size: 0,
            protocol: Protocol::Http,
        }
    }

    /// Check if response is successful (2xx)
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }

    /// Check if response is a redirect (3xx)
    pub fn is_redirect(&self) -> bool {
        (300..400).contains(&self.status)
    }

    /// Check if response is a client error (4xx)
    pub fn is_client_error(&self) -> bool {
        (400..500).contains(&self.status)
    }

    /// Check if response is a server error (5xx)
    pub fn is_server_error(&self) -> bool {
        (500..600).contains(&self.status)
    }

    /// Get a header value by name (case-insensitive)
    pub fn get_header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|h| h.key.eq_ignore_ascii_case(name) && h.enabled)
            .map(|h| h.value.as_str())
    }

    /// Get content type
    pub fn content_type(&self) -> Option<&str> {
        self.get_header("Content-Type")
    }

    /// Check if response is JSON
    pub fn is_json(&self) -> bool {
        self.content_type()
            .map(|ct| ct.contains("application/json"))
            .unwrap_or(false)
    }

    /// Format body as pretty JSON if possible
    pub fn pretty_body(&self) -> String {
        if self.is_json()
            && let Ok(value) = serde_json::from_str::<serde_json::Value>(&self.body)
                && let Ok(pretty) = serde_json::to_string_pretty(&value) {
                    return pretty;
                }
        self.body.clone()
    }

    /// Format response time
    pub fn format_time(&self) -> String {
        let ms = self.time.as_millis();
        if ms < 1000 {
            format!("{}ms", ms)
        } else {
            format!("{:.2}s", self.time.as_secs_f64())
        }
    }

    /// Format response size
    pub fn format_size(&self) -> String {
        if self.size < 1024 {
            format!("{} B", self.size)
        } else if self.size < 1024 * 1024 {
            format!("{:.1} KB", self.size as f64 / 1024.0)
        } else {
            format!("{:.1} MB", self.size as f64 / (1024.0 * 1024.0))
        }
    }
}

