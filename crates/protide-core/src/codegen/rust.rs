//! Rust code generation (using reqwest)

use super::CodegenRequest;

/// Generate Rust code using reqwest
pub fn generate_rust(request: &CodegenRequest) -> String {
    let mut lines = vec![
        "use reqwest;".to_string(),
        String::new(),
        "#[tokio::main]".to_string(),
        "async fn main() -> Result<(), reqwest::Error> {".to_string(),
    ];

    let has_body = request.body.as_ref().map(|b| !b.trim().is_empty()).unwrap_or(false);
    let is_json = request.headers.iter().any(|(k, v)| {
        k.eq_ignore_ascii_case("content-type") && v.contains("application/json")
    });

    // Build the request
    lines.push("    let client = reqwest::Client::new();".to_string());
    lines.push(String::new());

    let method = request.method.to_lowercase();
    lines.push(format!(
        "    let response = client.{}(\"{}\")",
        method,
        escape_rust_string(&request.url)
    ));

    // Add headers
    for (key, value) in &request.headers {
        lines.push(format!(
            "        .header(\"{}\", \"{}\")",
            escape_rust_string(key),
            escape_rust_string(value)
        ));
    }

    // Add body
    if has_body
        && let Some(body) = &request.body {
            if is_json {
                // Try to format as serde_json
                lines.push(format!("        .body(\"{}\")", escape_rust_string(body)));
            } else {
                lines.push(format!("        .body(\"{}\")", escape_rust_string(body)));
            }
        }

    lines.push("        .send()".to_string());
    lines.push("        .await?;".to_string());
    lines.push(String::new());
    lines.push("    println!(\"Status: {}\", response.status());".to_string());
    lines.push("    let body = response.text().await?;".to_string());
    lines.push("    println!(\"{}\", body);".to_string());
    lines.push(String::new());
    lines.push("    Ok(())".to_string());
    lines.push("}".to_string());

    lines.join("\n")
}

fn escape_rust_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_get() {
        let request = CodegenRequest::new("GET", "https://api.example.com/users");
        let code = generate_rust(&request);
        assert!(code.contains("use reqwest"));
        assert!(code.contains("#[tokio::main]"));
        assert!(code.contains("client.get("));
        assert!(code.contains(".send()"));
    }

    #[test]
    fn test_post_with_body() {
        let request = CodegenRequest::new("POST", "https://api.example.com/users")
            .with_headers(vec![("Content-Type".to_string(), "application/json".to_string())])
            .with_body(Some(r#"{"name": "John"}"#.to_string()));
        let code = generate_rust(&request);
        assert!(code.contains("client.post("));
        assert!(code.contains(".header(\"Content-Type\""));
        assert!(code.contains(".body("));
    }

    #[test]
    fn test_with_headers() {
        let request = CodegenRequest::new("GET", "https://api.example.com")
            .with_headers(vec![
                ("Authorization".to_string(), "Bearer token".to_string()),
            ]);
        let code = generate_rust(&request);
        assert!(code.contains(".header(\"Authorization\", \"Bearer token\")"));
    }

    #[test]
    fn test_url_injection_is_escaped() {
        // A malicious URL containing an unescaped double quote could break
        // out of the Rust string literal and inject arbitrary Rust code.
        let request = CodegenRequest::new(
            "GET",
            "https://example.com/\"); std::process::exit(1); (\"",
        );
        let code = generate_rust(&request);
        assert!(!code.contains("client.get(\"https://example.com/\"); std::process::exit(1); (\"\")"));
        assert!(code.contains("\\\""));
    }

    #[test]
    fn test_header_key_injection_is_escaped() {
        let request = CodegenRequest::new("GET", "https://example.com").with_headers(vec![(
            "X-Evil\")\n        .header(\"X-Injected".to_string(),
            "value".to_string(),
        )]);
        let code = generate_rust(&request);
        assert!(!code.contains(".header(\"X-Evil\")\n        .header(\"X-Injected\", \"value\")"));
    }
}
