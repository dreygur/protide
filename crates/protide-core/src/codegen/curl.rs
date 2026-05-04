//! cURL command generation

use super::CodegenRequest;

/// Generate a cURL command from a request
pub fn generate_curl(request: &CodegenRequest) -> String {
    let mut parts = vec!["curl".to_string()];

    // Method (GET is default, only add if not GET)
    if request.method != "GET" {
        parts.push(format!("-X {}", request.method));
    }

    // URL
    parts.push(format!("'{}'", request.url));

    // Headers
    for (key, value) in &request.headers {
        parts.push(format!("-H '{}: {}'", key, escape_single_quotes(value)));
    }

    // Body
    if let Some(body) = &request.body {
        if !body.trim().is_empty() {
            parts.push(format!("-d '{}'", escape_single_quotes(body)));
        }
    }

    parts.join(" \\\n  ")
}

fn escape_single_quotes(s: &str) -> String {
    s.replace('\'', "'\\''")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_get() {
        let request = CodegenRequest::new("GET", "https://api.example.com/users");
        let code = generate_curl(&request);
        assert!(code.contains("curl"));
        assert!(code.contains("https://api.example.com/users"));
        assert!(!code.contains("-X")); // GET is default
    }

    #[test]
    fn test_post_with_body() {
        let request = CodegenRequest::new("POST", "https://api.example.com/users")
            .with_headers(vec![("Content-Type".to_string(), "application/json".to_string())])
            .with_body(Some(r#"{"name": "John"}"#.to_string()));
        let code = generate_curl(&request);
        assert!(code.contains("-X POST"));
        assert!(code.contains("-H 'Content-Type: application/json'"));
        assert!(code.contains(r#"-d '{"name": "John"}'"#));
    }

    #[test]
    fn test_escape_single_quotes() {
        let request = CodegenRequest::new("POST", "https://api.example.com")
            .with_body(Some("It's a test".to_string()));
        let code = generate_curl(&request);
        assert!(code.contains("It'\\''s a test"));
    }
}
