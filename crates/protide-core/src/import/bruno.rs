//! Bruno (.bru) file format import
//!
//! Bruno uses a custom block-based format:
//!
//! ```text
//! meta {
//!   name: Get Users
//!   type: http
//!   seq: 1
//! }
//!
//! get {
//!   url: https://api.example.com/users
//!   body: none
//!   auth: none
//! }
//!
//! headers {
//!   Content-Type: application/json
//! }
//!
//! body:json {
//!   { "key": "value" }
//! }
//! ```

use http_parser::{HttpMethod, KeyValue, Request};
use super::ImportResult;

/// Parse a Bruno .bru file into an ImportResult.
pub fn parse_bruno(content: &str) -> Result<ImportResult, String> {
    let mut result = ImportResult::new();
    let blocks = parse_blocks(content);

    let mut name = String::new();
    let mut method = String::from("GET");
    let mut url = String::new();
    let mut headers: Vec<KeyValue> = Vec::new();
    let mut body = String::new();
    let mut query_params: Vec<KeyValue> = Vec::new();

    for (block_name, lines) in &blocks {
        match block_name.as_str() {
            "meta" => {
                for line in lines {
                    if let Some((k, v)) = parse_kv(line)
                        && k == "name" {
                            name = v;
                        }
                }
            }
            "get" | "post" | "put" | "delete" | "patch" | "head" | "options" => {
                method = block_name.to_uppercase();
                for line in lines {
                    if let Some((k, v)) = parse_kv(line)
                        && k == "url" {
                            url = v;
                        }
                }
            }
            "headers" => {
                for line in lines {
                    if let Some((k, v)) = parse_colon_kv(line) {
                        headers.push(KeyValue {
                            key: k,
                            value: v,
                            enabled: true,
                        });
                    }
                }
            }
            "query" => {
                for line in lines {
                    if let Some((k, v)) = parse_colon_kv(line) {
                        let enabled = !k.starts_with('~');
                        let key = k.trim_start_matches('~').to_string();
                        query_params.push(KeyValue { key, value: v, enabled });
                    }
                }
            }
            "body:json" | "body:text" | "body:xml" | "body:graphql" | "body:form-urlencoded" => {
                body = lines.join("\n").trim().to_string();
            }
            _ => {}
        }
    }

    if url.is_empty() {
        return Err("No URL found in Bruno file".to_string());
    }

    // Append query params to URL if any
    let final_url = if query_params.is_empty() {
        url
    } else {
        let qs: String = query_params
            .iter()
            .filter(|kv| kv.enabled)
            .map(|kv| format!("{}={}", urlencoding::encode(&kv.key), urlencoding::encode(&kv.value)))
            .collect::<Vec<_>>()
            .join("&");
        if url.contains('?') {
            format!("{}&{}", url, qs)
        } else {
            format!("{}?{}", url, qs)
        }
    };

    let http_method = HttpMethod::from_str(&method).unwrap_or(HttpMethod::Get);

    let mut request = Request::new(http_method, final_url);
    if !name.is_empty() {
        request.meta.name = Some(name);
    }
    request.headers = headers;
    request.body = if body.is_empty() { None } else { Some(body) };

    result.requests.push(request);
    Ok(result)
}

/// Split .bru file into named blocks: `(block_name, lines_inside)`.
fn parse_blocks(content: &str) -> Vec<(String, Vec<String>)> {
    let mut blocks: Vec<(String, Vec<String>)> = Vec::new();
    let mut current_name: Option<String> = None;
    let mut current_lines: Vec<String> = Vec::new();
    let mut depth = 0usize;

    for line in content.lines() {
        let trimmed = line.trim();

        if current_name.is_none() {
            // Look for block start: "name {" or "name:variant {"
            if let Some(block_name) = trimmed.strip_suffix('{').map(|s| s.trim().to_string())
                && !block_name.is_empty() {
                    current_name = Some(block_name);
                    current_lines.clear();
                    depth = 1;
                }
        } else {
            if trimmed.ends_with('{') {
                depth += 1;
                current_lines.push(line.to_string());
            } else if trimmed == "}" {
                depth -= 1;
                if depth == 0 {
                    blocks.push((current_name.take().unwrap(), current_lines.clone()));
                    current_lines.clear();
                } else {
                    current_lines.push(line.to_string());
                }
            } else {
                current_lines.push(line.to_string());
            }
        }
    }

    blocks
}

/// Parse `key: value` (Bruno meta format)
fn parse_kv(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    let (k, v) = trimmed.split_once(':')?;
    Some((k.trim().to_string(), v.trim().to_string()))
}

/// Parse `Key: Value` (header/query format — value may contain colons)
fn parse_colon_kv(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    let (k, v) = trimmed.split_once(':')?;
    Some((k.trim().to_string(), v.trim().to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
meta {
  name: Get Users
  type: http
  seq: 1
}

get {
  url: https://api.example.com/users
  body: none
  auth: none
}

headers {
  Content-Type: application/json
  X-API-Key: secret
}
"#;

    #[test]
    fn test_parse_bruno_basic() {
        let result = parse_bruno(SAMPLE).unwrap();
        assert_eq!(result.requests.len(), 1);
        let req = &result.requests[0];
        assert_eq!(req.url, "https://api.example.com/users");
        assert_eq!(req.meta.name.as_deref(), Some("Get Users"));
        assert_eq!(req.headers.len(), 2);
    }

    #[test]
    fn test_parse_blocks() {
        let blocks = parse_blocks(SAMPLE);
        let names: Vec<&str> = blocks.iter().map(|(n, _)| n.as_str()).collect();
        assert!(names.contains(&"meta"));
        assert!(names.contains(&"get"));
        assert!(names.contains(&"headers"));
    }
}
