//! JavaScript code generation (using fetch API)

use super::CodegenRequest;

/// Generate JavaScript code using the fetch API
pub fn generate_javascript(request: &CodegenRequest) -> String {
    let mut lines = Vec::new();

    // Headers object
    let has_headers = !request.headers.is_empty();
    let has_body = request.body.as_ref().map(|b| !b.trim().is_empty()).unwrap_or(false);

    // Options object
    lines.push("const options = {".to_string());
    lines.push(format!("  method: '{}',", request.method));

    if has_headers {
        lines.push("  headers: {".to_string());
        for (key, value) in &request.headers {
            lines.push(format!("    '{}': '{}',", key, escape_js_string(value)));
        }
        lines.push("  },".to_string());
    }

    if has_body
        && let Some(body) = &request.body {
            // Check if it's JSON
            let is_json = request.headers.iter().any(|(k, v)| {
                k.eq_ignore_ascii_case("content-type") && v.contains("application/json")
            });

            if is_json {
                // Try to format as JavaScript object
                if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(body) {
                    lines.push(format!("  body: JSON.stringify({}),", json_to_js(&json_val, 2)));
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
    lines.push(format!("fetch('{}', options)", request.url));
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
                    .map(|(k, v)| format!("{}{}: {}", spaces, k, json_to_js(v, 0)))
                    .collect();
                format!("{{\n{}\n{}}}", items.join(",\n"), " ".repeat(indent.saturating_sub(2)))
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
            .with_headers(vec![("Content-Type".to_string(), "application/json".to_string())])
            .with_body(Some(r#"{"name": "John"}"#.to_string()));
        let code = generate_javascript(&request);
        assert!(code.contains("method: 'POST'"));
        assert!(code.contains("JSON.stringify"));
        assert!(code.contains("name: 'John'"));
    }

    #[test]
    fn test_with_headers() {
        let request = CodegenRequest::new("GET", "https://api.example.com")
            .with_headers(vec![
                ("Authorization".to_string(), "Bearer token".to_string()),
            ]);
        let code = generate_javascript(&request);
        assert!(code.contains("headers: {"));
        assert!(code.contains("'Authorization': 'Bearer token'"));
    }
}
