//! Python code generation (using requests library)

use super::CodegenRequest;

/// Generate Python code using the requests library
pub fn generate_python(request: &CodegenRequest) -> String {
    let mut lines = vec!["import requests".to_string(), String::new()];

    // URL
    lines.push(format!("url = \"{}\"", escape_python_string(&request.url)));

    // Headers
    if !request.headers.is_empty() {
        lines.push("headers = {".to_string());
        for (key, value) in &request.headers {
            lines.push(format!(
                "    \"{}\": \"{}\",",
                escape_python_string(key),
                escape_python_string(value)
            ));
        }
        lines.push("}".to_string());
    }

    // Body
    let has_body = request
        .body
        .as_ref()
        .map(|b| !b.trim().is_empty())
        .unwrap_or(false);
    let is_json = request
        .headers
        .iter()
        .any(|(k, v)| k.eq_ignore_ascii_case("content-type") && v.contains("application/json"));

    if has_body {
        if is_json {
            lines.push("import json".to_string());
            lines.push(String::new());
            // Try to format as Python dict
            if let Some(body) = &request.body {
                if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(body) {
                    lines.push(format!("data = {}", json_to_python(&json_val)));
                } else {
                    lines.push(format!("data = \"{}\"", escape_python_string(body)));
                }
            }
        } else if let Some(body) = &request.body {
            lines.push(format!("data = \"{}\"", escape_python_string(body)));
        }
    }

    lines.push(String::new());

    // Request call
    // Use the generic `requests.request()` form and pass the method as a
    // quoted/escaped string argument rather than splicing it as a bareword
    // method name (e.g. `requests.get(...)`). The method can be arbitrary
    // free text (HttpMethod::Custom), so treating it as an identifier would
    // allow Python code injection.
    let mut call = format!(
        "response = requests.request(\"{}\", url",
        escape_python_string(&request.method)
    );

    if !request.headers.is_empty() {
        call.push_str(", headers=headers");
    }

    if has_body {
        if is_json {
            call.push_str(", json=data");
        } else {
            call.push_str(", data=data");
        }
    }

    call.push(')');
    lines.push(call);

    // Print response
    lines.push(String::new());
    lines.push("print(response.status_code)".to_string());
    lines.push("print(response.text)".to_string());

    lines.join("\n")
}

fn escape_python_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn json_to_python(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "None".to_string(),
        serde_json::Value::Bool(b) => if *b { "True" } else { "False" }.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => format!("\"{}\"", escape_python_string(s)),
        serde_json::Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(json_to_python).collect();
            format!("[{}]", items.join(", "))
        }
        serde_json::Value::Object(obj) => {
            let items: Vec<String> = obj
                .iter()
                .map(|(k, v)| format!("\"{}\": {}", k, json_to_python(v)))
                .collect();
            format!("{{{}}}", items.join(", "))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_get() {
        let request = CodegenRequest::new("GET", "https://api.example.com/users");
        let code = generate_python(&request);
        assert!(code.contains("import requests"));
        assert!(code.contains("requests.request(\"GET\", url)"));
        assert!(code.contains("print(response.status_code)"));
    }

    #[test]
    fn test_post_with_json() {
        let request = CodegenRequest::new("POST", "https://api.example.com/users")
            .with_headers(vec![(
                "Content-Type".to_string(),
                "application/json".to_string(),
            )])
            .with_body(Some(r#"{"name": "John", "active": true}"#.to_string()));
        let code = generate_python(&request);
        assert!(code.contains("requests.request(\"POST\", url"));
        assert!(code.contains("json=data"));
        assert!(code.contains("\"name\": \"John\""));
        assert!(code.contains("\"active\": True"));
    }

    #[test]
    fn test_url_injection_is_escaped() {
        // A malicious URL containing an unescaped quote could break out of
        // the Python string literal and inject arbitrary code.
        let request = CodegenRequest::new(
            "GET",
            "https://example.com/\"; import os; os.system(\"rm -rf ~\"); \"",
        );
        let code = generate_python(&request);
        assert!(!code.contains("url = \"https://example.com/\"; import os"));
        assert!(code.contains(
            "url = \"https://example.com/\\\"; import os; os.system(\\\"rm -rf ~\\\"); \\\"\""
        ));
    }

    #[test]
    fn test_method_injection_uses_generic_request_call() {
        // HttpMethod::Custom allows arbitrary free text. Splicing it as a
        // bareword `requests.<method>(...)` call name would allow arbitrary
        // attribute/code injection, so it must be passed as a quoted string
        // argument to `requests.request()` instead.
        let request = CodegenRequest::new(
            "GET\"); import os; os.system(\"rm -rf ~", // attempted breakout
            "https://example.com",
        );
        let code = generate_python(&request);
        assert!(!code.contains("requests.GET"));
        assert!(!code.contains("os.system(\"rm -rf ~\")(url"));
        assert!(code.contains("requests.request(\""));
    }

    #[test]
    fn test_header_key_injection_is_escaped() {
        let request = CodegenRequest::new("GET", "https://example.com").with_headers(vec![(
            "X-Evil\": \"1\", \"X-Injected".to_string(),
            "value".to_string(),
        )]);
        let code = generate_python(&request);
        assert!(!code.contains("\"X-Evil\": \"1\", \"X-Injected\": \"value\","));
    }

    #[test]
    fn test_json_to_python_conversion() {
        let json: serde_json::Value = serde_json::json!({
            "name": "test",
            "count": 42,
            "active": true,
            "tags": ["a", "b"]
        });
        let python = json_to_python(&json);
        assert!(python.contains("\"name\": \"test\""));
        assert!(python.contains("\"count\": 42"));
        assert!(python.contains("\"active\": True"));
    }
}
