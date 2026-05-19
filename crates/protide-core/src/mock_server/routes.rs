//! Mock route definitions and matching

use std::collections::HashMap;

/// HTTP methods for mock routes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
    Any,
}

impl HttpMethod {
    /// Check if method matches
    pub fn matches(&self, method: &str) -> bool {
        match self {
            HttpMethod::Any => true,
            HttpMethod::Get => method.eq_ignore_ascii_case("GET"),
            HttpMethod::Post => method.eq_ignore_ascii_case("POST"),
            HttpMethod::Put => method.eq_ignore_ascii_case("PUT"),
            HttpMethod::Patch => method.eq_ignore_ascii_case("PATCH"),
            HttpMethod::Delete => method.eq_ignore_ascii_case("DELETE"),
            HttpMethod::Head => method.eq_ignore_ascii_case("HEAD"),
            HttpMethod::Options => method.eq_ignore_ascii_case("OPTIONS"),
        }
    }

    /// Get display name
    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Delete => "DELETE",
            HttpMethod::Head => "HEAD",
            HttpMethod::Options => "OPTIONS",
            HttpMethod::Any => "ANY",
        }
    }

    /// All methods for UI dropdown
    pub fn all() -> &'static [HttpMethod] {
        &[
            HttpMethod::Any,
            HttpMethod::Get,
            HttpMethod::Post,
            HttpMethod::Put,
            HttpMethod::Patch,
            HttpMethod::Delete,
            HttpMethod::Head,
            HttpMethod::Options,
        ]
    }
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Mock response configuration
#[derive(Debug, Clone)]
pub struct MockResponse {
    /// HTTP status code
    pub status: u16,
    /// Response headers
    pub headers: HashMap<String, String>,
    /// Response body
    pub body: String,
    /// Optional delay in milliseconds
    pub delay_ms: u64,
}

impl MockResponse {
    /// Create a new mock response
    pub fn new(status: u16, body: impl Into<String>) -> Self {
        Self {
            status,
            headers: HashMap::new(),
            body: body.into(),
            delay_ms: 0,
        }
    }

    /// Create a 200 OK response
    pub fn ok(body: impl Into<String>) -> Self {
        Self::new(200, body)
    }

    /// Create a 201 Created response
    pub fn created(body: impl Into<String>) -> Self {
        Self::new(201, body)
    }

    /// Create a 204 No Content response
    pub fn no_content() -> Self {
        Self::new(204, "")
    }

    /// Create a 400 Bad Request response
    pub fn bad_request(body: impl Into<String>) -> Self {
        Self::new(400, body)
    }

    /// Create a 404 Not Found response
    pub fn not_found(body: impl Into<String>) -> Self {
        Self::new(404, body)
    }

    /// Create a 500 Internal Server Error response
    pub fn server_error(body: impl Into<String>) -> Self {
        Self::new(500, body)
    }

    /// Add a header
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Set JSON content type
    pub fn json(self) -> Self {
        self.with_header("Content-Type", "application/json")
    }

    /// Set delay
    pub fn with_delay(mut self, delay_ms: u64) -> Self {
        self.delay_ms = delay_ms;
        self
    }
}

impl Default for MockResponse {
    fn default() -> Self {
        Self::ok("")
    }
}

/// A mock route definition
#[derive(Debug, Clone)]
pub struct MockRoute {
    /// Route name for display
    pub name: String,
    /// HTTP method to match
    pub method: HttpMethod,
    /// Path pattern (supports * wildcard)
    pub path: String,
    /// Response to return (used when proxy_target is None)
    pub response: MockResponse,
    /// Whether route is enabled
    pub enabled: bool,
    /// Optional proxy target URL — when set, the request is forwarded here
    /// instead of returning the static response.
    /// E.g. "https://api.example.com" → request to /users becomes https://api.example.com/users
    pub proxy_target: Option<String>,
}

impl MockRoute {
    /// Create a new mock route
    pub fn new(method: HttpMethod, path: impl Into<String>, response: MockResponse) -> Self {
        let path = path.into();
        Self {
            name: format!("{} {}", method, path),
            method,
            path,
            response,
            enabled: true,
            proxy_target: None,
        }
    }

    /// Create a proxy route that forwards requests to `target_url`.
    pub fn proxy(method: HttpMethod, path: impl Into<String>, target_url: impl Into<String>) -> Self {
        let path = path.into();
        let target = target_url.into();
        Self {
            name: format!("{} {} → {}", method, path, target),
            method,
            path,
            response: MockResponse::default(),
            enabled: true,
            proxy_target: Some(target),
        }
    }

    /// Whether this route uses proxy forwarding
    pub fn is_proxy(&self) -> bool {
        self.proxy_target.is_some()
    }

    /// Set route name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Check if this route matches the given method and path
    pub fn matches(&self, method: &str, path: &str) -> bool {
        if !self.enabled {
            return false;
        }

        if !self.method.matches(method) {
            return false;
        }

        self.path_matches(path)
    }

    /// Check if path matches the pattern
    fn path_matches(&self, path: &str) -> bool {
        let pattern = &self.path;

        // Exact match
        if pattern == path {
            return true;
        }

        // Wildcard matching
        if pattern.contains('*') {
            return self.wildcard_match(pattern, path);
        }

        // Path with or without trailing slash
        if pattern.ends_with('/') {
            pattern.trim_end_matches('/') == path
        } else {
            format!("{}/", pattern) == path
        }
    }

    /// Simple wildcard matching
    fn wildcard_match(&self, pattern: &str, path: &str) -> bool {
        let parts: Vec<&str> = pattern.split('*').collect();

        if parts.len() == 1 {
            return pattern == path;
        }

        let mut remaining = path;

        // Check prefix (before first *)
        if !parts[0].is_empty() {
            if !remaining.starts_with(parts[0]) {
                return false;
            }
            remaining = &remaining[parts[0].len()..];
        }

        // Check suffix (after last *)
        let last = *parts.last().unwrap();
        if !last.is_empty() {
            if !remaining.ends_with(last) {
                return false;
            }
            remaining = &remaining[..remaining.len() - last.len()];
        }

        // Check middle parts in order (greedy search)
        for part in &parts[1..parts.len() - 1] {
            if part.is_empty() {
                continue;
            }
            if let Some(idx) = remaining.find(part) {
                remaining = &remaining[idx + part.len()..];
            } else {
                return false;
            }
        }

        true
    }
}

impl Default for MockRoute {
    fn default() -> Self {
        Self {
            name: "New Route".to_string(),
            method: HttpMethod::Get,
            path: "/".to_string(),
            response: MockResponse::ok(""),
            enabled: true,
            proxy_target: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_method_matches() {
        assert!(HttpMethod::Get.matches("GET"));
        assert!(HttpMethod::Get.matches("get"));
        assert!(!HttpMethod::Get.matches("POST"));
        assert!(HttpMethod::Any.matches("GET"));
        assert!(HttpMethod::Any.matches("POST"));
    }

    #[test]
    fn test_path_exact_match() {
        let route = MockRoute::new(HttpMethod::Get, "/api/users", MockResponse::ok("[]"));
        assert!(route.matches("GET", "/api/users"));
        assert!(!route.matches("GET", "/api/posts"));
    }

    #[test]
    fn test_path_wildcard() {
        let route = MockRoute::new(HttpMethod::Get, "/api/*", MockResponse::ok(""));
        assert!(route.matches("GET", "/api/users"));
        assert!(route.matches("GET", "/api/posts"));
        assert!(!route.matches("GET", "/other"));
    }

    #[test]
    fn test_path_wildcard_middle() {
        let route = MockRoute::new(HttpMethod::Get, "/api/*/details", MockResponse::ok(""));
        assert!(route.matches("GET", "/api/users/details"));
        assert!(route.matches("GET", "/api/123/details"));
    }

    #[test]
    fn test_disabled_route() {
        let mut route = MockRoute::new(HttpMethod::Get, "/test", MockResponse::ok(""));
        route.enabled = false;
        assert!(!route.matches("GET", "/test"));
    }

    #[test]
    fn test_response_builder() {
        let response = MockResponse::ok(r#"{"id": 1}"#)
            .json()
            .with_delay(100);

        assert_eq!(response.status, 200);
        assert_eq!(response.headers.get("Content-Type"), Some(&"application/json".to_string()));
        assert_eq!(response.delay_ms, 100);
    }
}
