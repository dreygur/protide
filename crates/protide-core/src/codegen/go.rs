//! Go code generation (using net/http)

use super::CodegenRequest;

/// Generate Go code using net/http
pub fn generate_go(request: &CodegenRequest) -> String {
    let mut lines = vec![
        "package main".to_string(),
        String::new(),
        "import (".to_string(),
        "    \"fmt\"".to_string(),
        "    \"io\"".to_string(),
        "    \"net/http\"".to_string(),
    ];

    let has_body = request.body.as_ref().map(|b| !b.trim().is_empty()).unwrap_or(false);
    if has_body {
        lines.push("    \"strings\"".to_string());
    }

    lines.push(")".to_string());
    lines.push(String::new());
    lines.push("func main() {".to_string());

    // Create request body
    if has_body {
        if let Some(body) = &request.body {
            lines.push(format!("    body := strings.NewReader(\"{}\")", escape_go_string(body)));
            lines.push(String::new());
            lines.push(format!(
                "    req, err := http.NewRequest(\"{}\", \"{}\", body)",
                request.method, request.url
            ));
        }
    } else {
        lines.push(format!(
            "    req, err := http.NewRequest(\"{}\", \"{}\", nil)",
            request.method, request.url
        ));
    }

    lines.push("    if err != nil {".to_string());
    lines.push("        panic(err)".to_string());
    lines.push("    }".to_string());

    // Add headers
    if !request.headers.is_empty() {
        lines.push(String::new());
        for (key, value) in &request.headers {
            lines.push(format!("    req.Header.Set(\"{}\", \"{}\")", key, escape_go_string(value)));
        }
    }

    // Execute request
    lines.push(String::new());
    lines.push("    client := &http.Client{}".to_string());
    lines.push("    resp, err := client.Do(req)".to_string());
    lines.push("    if err != nil {".to_string());
    lines.push("        panic(err)".to_string());
    lines.push("    }".to_string());
    lines.push("    defer resp.Body.Close()".to_string());
    lines.push(String::new());
    lines.push("    respBody, err := io.ReadAll(resp.Body)".to_string());
    lines.push("    if err != nil {".to_string());
    lines.push("        panic(err)".to_string());
    lines.push("    }".to_string());
    lines.push(String::new());
    lines.push("    fmt.Println(\"Status:\", resp.Status)".to_string());
    lines.push("    fmt.Println(string(respBody))".to_string());
    lines.push("}".to_string());

    lines.join("\n")
}

fn escape_go_string(s: &str) -> String {
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
        let code = generate_go(&request);
        assert!(code.contains("package main"));
        assert!(code.contains("http.NewRequest(\"GET\""));
        assert!(code.contains("client.Do(req)"));
    }

    #[test]
    fn test_post_with_body() {
        let request = CodegenRequest::new("POST", "https://api.example.com/users")
            .with_headers(vec![("Content-Type".to_string(), "application/json".to_string())])
            .with_body(Some(r#"{"name": "John"}"#.to_string()));
        let code = generate_go(&request);
        assert!(code.contains("strings.NewReader"));
        assert!(code.contains("http.NewRequest(\"POST\""));
        assert!(code.contains("req.Header.Set(\"Content-Type\""));
    }
}
