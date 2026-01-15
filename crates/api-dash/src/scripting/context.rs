//! Script execution context with request/response data

use std::collections::HashMap;
use super::results::{TestResult, ModifiedRequest};

/// Request data accessible in scripts
#[derive(Debug, Clone, Default)]
pub struct RequestData {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

/// Response data accessible in scripts
#[derive(Debug, Clone)]
pub struct ResponseData {
    pub status: u16,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub time_ms: u64,
    pub size: usize,
}

/// Mutable context passed to scripts
#[derive(Debug, Clone, Default)]
pub struct ScriptContext {
    /// Request data (mutable in pre-script)
    pub request: RequestData,
    /// Response data (available in post-script/tests)
    pub response: Option<ResponseData>,
    /// Environment variables
    pub env: HashMap<String, String>,
    /// Test results collector
    pub test_results: Vec<TestResult>,
    /// Console output collector
    pub console_output: Vec<String>,
    /// Environment changes to persist
    pub env_changes: Vec<(String, String)>,
    /// Request modifications (from pre-script)
    pub modified_request: ModifiedRequest,
}

impl ScriptContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_request(mut self, request: RequestData) -> Self {
        self.request = request;
        self
    }

    pub fn with_env(mut self, env: HashMap<String, String>) -> Self {
        self.env = env;
        self
    }

    pub fn set_response(&mut self, response: ResponseData) {
        self.response = Some(response);
    }

    /// Get environment variable
    pub fn get_env(&self, name: &str) -> Option<&String> {
        self.env.get(name)
    }

    /// Set environment variable (persists after script)
    pub fn set_env(&mut self, name: String, value: String) {
        self.env.insert(name.clone(), value.clone());
        self.env_changes.push((name, value));
    }

    /// Check if environment variable exists
    pub fn has_env(&self, name: &str) -> bool {
        self.env.contains_key(name)
    }

    /// Remove environment variable
    pub fn remove_env(&mut self, name: &str) {
        self.env.remove(name);
    }

    /// Add console output
    pub fn log(&mut self, message: String) {
        self.console_output.push(message);
    }

    /// Add test result
    pub fn add_test_result(&mut self, result: TestResult) {
        self.test_results.push(result);
    }

    /// Modify request URL (pre-script only)
    pub fn set_url(&mut self, url: String) {
        self.request.url = url.clone();
        self.modified_request.url = Some(url);
    }

    /// Set request header (pre-script only)
    pub fn set_header(&mut self, name: String, value: String) {
        self.request.headers.insert(name.clone(), value.clone());
        self.modified_request.headers_to_set.push((name, value));
    }

    /// Remove request header (pre-script only)
    pub fn remove_header(&mut self, name: &str) {
        self.request.headers.remove(name);
        self.modified_request.headers_to_remove.push(name.to_string());
    }

    /// Set request body (pre-script only)
    pub fn set_body(&mut self, body: String) {
        self.request.body = Some(body.clone());
        self.modified_request.body = Some(body);
    }
}

impl RequestData {
    pub fn new(method: &str, url: &str) -> Self {
        Self {
            method: method.to_string(),
            url: url.to_string(),
            headers: HashMap::new(),
            body: None,
        }
    }

    pub fn with_headers(mut self, headers: Vec<(String, String)>) -> Self {
        for (k, v) in headers {
            self.headers.insert(k, v);
        }
        self
    }

    pub fn with_body(mut self, body: String) -> Self {
        self.body = Some(body);
        self
    }
}

impl ResponseData {
    pub fn new(status: u16, status_text: &str, body: String) -> Self {
        Self {
            status,
            status_text: status_text.to_string(),
            headers: HashMap::new(),
            body,
            time_ms: 0,
            size: 0,
        }
    }

    pub fn with_headers(mut self, headers: Vec<(String, String)>) -> Self {
        for (k, v) in headers {
            self.headers.insert(k.to_lowercase(), v);
        }
        self
    }

    pub fn with_time(mut self, time_ms: u64) -> Self {
        self.time_ms = time_ms;
        self
    }

    pub fn with_size(mut self, size: usize) -> Self {
        self.size = size;
        self
    }

    /// Get header value (case-insensitive)
    pub fn get_header(&self, name: &str) -> Option<&String> {
        self.headers.get(&name.to_lowercase())
    }

    /// Parse body as JSON
    pub fn json(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::from_str(&self.body)
    }
}
