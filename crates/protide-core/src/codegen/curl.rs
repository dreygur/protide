//! cURL command generation

use super::CodegenRequest;

/// Generate a cURL command from a request
pub fn generate_curl(request: &CodegenRequest) -> String {
    let mut parts = vec!["curl".to_string()];

    // Method (GET is default, only add if not GET)
    if request.method != "GET" {
        parts.push(format!("-X '{}'", escape_single_quotes(&request.method)));
    }

    // URL
    parts.push(format!("'{}'", escape_single_quotes(&request.url)));

    // Headers
    for (key, value) in &request.headers {
        parts.push(format!(
            "-H '{}: {}'",
            escape_single_quotes(key),
            escape_single_quotes(value)
        ));
    }

    // Body
    if let Some(body) = &request.body
        && !body.trim().is_empty()
    {
        parts.push(format!("-d '{}'", escape_single_quotes(body)));
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
            .with_headers(vec![(
                "Content-Type".to_string(),
                "application/json".to_string(),
            )])
            .with_body(Some(r#"{"name": "John"}"#.to_string()));
        let code = generate_curl(&request);
        // Method is now quoted+escaped like everything else (see the
        // injection-escaping fix below), so it reads `-X 'POST'`, not the
        // old bare `-X POST`.
        assert!(code.contains("-X 'POST'"));
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

    #[test]
    fn test_url_injection_is_escaped() {
        // A malicious URL containing a single quote could break out of the
        // quoted argument and inject arbitrary shell commands.
        let request = CodegenRequest::new("GET", "https://example.com/'; rm -rf ~; echo '");
        let code = generate_curl(&request);
        // Note: correctly-escaped output for this payload necessarily
        // *contains* the substring "'; rm -rf ~; echo '" as an artifact of
        // the close-escape-reopen ('\'') single-quote escaping technique
        // itself - that substring appearing is not evidence of a bug. The
        // only thing that actually proves the payload is inert is that the
        // whole argument reconstructs to the exact expected safely-escaped
        // form below, with the URL still wrapped in a single quoted shell
        // argument throughout.
        assert!(code.contains("'https://example.com/'\\''; rm -rf ~; echo '\\'''"));
    }

    #[test]
    fn test_method_injection_is_escaped() {
        // HttpMethod::Custom allows arbitrary free text, so the method must
        // be quoted and escaped just like headers/body.
        let request = CodegenRequest::new("GET'; rm -rf ~; echo '", "https://example.com");
        let code = generate_curl(&request);
        assert!(!code.contains("-X GET'; rm -rf ~; echo '"));
        assert!(code.contains("-X 'GET'\\''; rm -rf ~; echo '\\'''"));
    }

    #[test]
    fn test_header_key_injection_is_escaped() {
        let request = CodegenRequest::new("GET", "https://example.com").with_headers(vec![(
            "X-Evil' -H 'X-Injected: yes".to_string(),
            "value".to_string(),
        )]);
        let code = generate_curl(&request);
        assert!(!code.contains("-H 'X-Evil' -H 'X-Injected: yes: value'"));
    }

    #[test]
    fn test_header_value_injection_is_escaped() {
        let request = CodegenRequest::new("GET", "https://example.com").with_headers(vec![(
            "X-Token".to_string(),
            "v' ; rm -rf ~ ; echo '".to_string(),
        )]);
        let code = generate_curl(&request);
        assert!(
            !code.contains("-H 'X-Token: v' ; rm -rf ~ ; echo ''"),
            "header value escaped the quoted argument: {}",
            code
        );
        assert!(code.contains("-H 'X-Token: v'\\'' ; rm -rf ~ ; echo '\\'''"));
    }

    #[test]
    fn test_body_injection_is_escaped() {
        let request = CodegenRequest::new("POST", "https://example.com").with_body(Some(
            "payload'; curl https://evil.example; echo '".to_string(),
        ));
        let code = generate_curl(&request);
        assert!(
            !code.contains("-d 'payload'; curl https://evil.example; echo ''"),
            "body escaped the quoted argument: {}",
            code
        );
        assert!(code.contains("-d 'payload'\\''; curl https://evil.example; echo '\\'''"));
    }

    #[test]
    fn test_query_params_in_url_are_escaped() {
        let request = CodegenRequest::new("GET", "https://example.com/search?q='$(id)'&sort=asc");
        let code = generate_curl(&request);
        // Every single quote from the URL must be neutralised by the
        // close-escape-reopen sequence, so no bare `'` survives to end the
        // quoted argument early.
        assert!(code.contains("'\\''$(id)'\\''&sort=asc'"), "{}", code);
    }

    #[test]
    fn test_backslash_in_body_is_not_a_line_continuation() {
        // Arguments are joined with " \\\n  ", so a trailing backslash in a
        // value must stay inside its single-quoted argument (single quotes
        // do not honour backslash escapes in POSIX shells).
        let request = CodegenRequest::new("POST", "https://example.com")
            .with_body(Some("ends-with-backslash\\".to_string()));
        let code = generate_curl(&request);
        assert!(code.ends_with("-d 'ends-with-backslash\\'"), "{}", code);
    }
}
