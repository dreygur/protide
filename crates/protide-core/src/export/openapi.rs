//! Export a collection directory as an OpenAPI 3.0 JSON specification

use std::collections::HashSet;
use std::path::Path;
use serde_json::{Value, json};

/// Walk a collection root and produce an OpenAPI 3.0 JSON spec string.
pub fn export_openapi(root: &Path) -> Result<String, String> {
    let title = root
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("API Collection");

    let mut paths: serde_json::Map<String, Value> = serde_json::Map::new();
    let mut servers: Vec<String> = Vec::new();
    let mut seen_servers: HashSet<String> = HashSet::new();

    collect_dir(root, &mut paths, &mut servers, &mut seen_servers)?;

    let servers_val: Vec<Value> = servers.iter()
        .map(|s| json!({ "url": s }))
        .collect();

    let spec = json!({
        "openapi": "3.0.0",
        "info": { "title": title, "version": "1.0.0" },
        "servers": servers_val,
        "paths": paths
    });

    serde_json::to_string_pretty(&spec).map_err(|e| e.to_string())
}

fn collect_dir(
    dir: &Path,
    paths: &mut serde_json::Map<String, Value>,
    servers: &mut Vec<String>,
    seen_servers: &mut HashSet<String>,
) -> Result<(), String> {
    let mut entries: Vec<std::fs::DirEntry> = std::fs::read_dir(dir)
        .map_err(|e| format!("Cannot read directory {}: {}", dir.display(), e))?
        .filter_map(|e| e.ok())
        .collect();

    entries.sort_by_key(|e| {
        let path = e.path();
        (!path.is_dir(), e.file_name().to_string_lossy().to_lowercase())
    });

    for entry in entries {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') { continue; }

        if path.is_dir() {
            collect_dir(&path, paths, servers, seen_servers)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("http") {
            if let Ok(content) = std::fs::read_to_string(&path) {
                add_http_requests(&content, paths, servers, seen_servers);
            }
        }
    }

    Ok(())
}

fn add_http_requests(
    content: &str,
    paths: &mut serde_json::Map<String, Value>,
    servers: &mut Vec<String>,
    seen_servers: &mut HashSet<String>,
) {
    let requests = match http_parser::parse(content) {
        Ok(r) if !r.is_empty() => r,
        _ => return,
    };

    for req in &requests {
        let (server, oas_path, query_params) = split_url(&req.url);

        if !server.is_empty() && seen_servers.insert(server.clone()) {
            servers.push(server);
        }

        let method = req.method.as_str().to_lowercase();

        let operation = build_operation(req, query_params);

        let path_entry = paths
            .entry(oas_path)
            .or_insert_with(|| Value::Object(serde_json::Map::new()));

        if let Value::Object(obj) = path_entry {
            obj.insert(method, operation);
        }
    }
}

/// Split a URL into (server_origin, oas_path, query_params).
fn split_url(url: &str) -> (String, String, Vec<(String, String)>) {
    let (server, rest) = if url.starts_with("http://") || url.starts_with("https://") {
        let prefix = if url.starts_with("https://") { 8 } else { 7 };
        let path_start = url[prefix..].find('/').map(|i| i + prefix).unwrap_or(url.len());
        (url[..path_start].to_string(), &url[path_start..])
    } else {
        (String::new(), url)
    };

    let (raw_path, query_str) = if let Some(q) = rest.find('?') {
        (&rest[..q], Some(&rest[q + 1..]))
    } else {
        (rest, None)
    };

    // {{var}} → {var}
    let oas_path = raw_path.replace("{{", "{").replace("}}", "}");

    let query_params = query_str
        .unwrap_or("")
        .split('&')
        .filter(|s| !s.is_empty())
        .filter_map(|kv| {
            let mut parts = kv.splitn(2, '=');
            let k = parts.next()?.to_string();
            let v = parts.next().unwrap_or("").to_string();
            Some((k, v))
        })
        .collect();

    (server, oas_path, query_params)
}

fn build_operation(req: &http_parser::Request, query_params: Vec<(String, String)>) -> Value {
    let mut parameters: Vec<Value> = Vec::new();

    // Header parameters (skip Content-Type - goes in requestBody)
    for h in req.headers.iter().filter(|h| h.enabled && !h.key.eq_ignore_ascii_case("content-type")) {
        parameters.push(json!({
            "in": "header",
            "name": h.key,
            "schema": { "type": "string" },
            "example": h.value
        }));
    }

    // Query parameters extracted from URL
    for (k, v) in query_params {
        let name = k.replace('{', "{{").replace('}', "}}"); // restore protide syntax for display
        let schema_type = if v.parse::<i64>().is_ok() { "integer" }
            else if v == "true" || v == "false" { "boolean" }
            else { "string" };
        parameters.push(json!({
            "in": "query",
            "name": name,
            "schema": { "type": schema_type },
            "example": v
        }));
    }

    let mut op = serde_json::Map::new();

    if let Some(name) = &req.meta.name {
        op.insert("operationId".to_string(), Value::String(name.clone()));
    }
    if let Some(desc) = &req.meta.description {
        op.insert("description".to_string(), Value::String(desc.clone()));
    }
    if !parameters.is_empty() {
        op.insert("parameters".to_string(), Value::Array(parameters));
    }

    if let Some(body) = &req.body {
        let trimmed = body.trim();
        if !trimmed.is_empty() {
            let ct = req.headers.iter()
                .find(|h| h.enabled && h.key.eq_ignore_ascii_case("content-type"))
                .map(|h| h.value.as_str())
                .unwrap_or_else(|| detect_content_type(trimmed));

            let content_value = if ct.contains("json") {
                if let Ok(parsed) = serde_json::from_str::<Value>(trimmed) {
                    json!({ "example": parsed })
                } else {
                    json!({ "example": trimmed })
                }
            } else {
                json!({ "example": trimmed })
            };

            op.insert("requestBody".to_string(), json!({
                "required": true,
                "content": { ct: content_value }
            }));
        }
    }

    op.insert("responses".to_string(), json!({
        "200": { "description": "OK" }
    }));

    Value::Object(op)
}

fn detect_content_type(body: &str) -> &'static str {
    if body.starts_with('{') || body.starts_with('[') { "application/json" }
    else if body.starts_with('<') { "application/xml" }
    else { "text/plain" }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_url_simple() {
        let (server, path, query) = split_url("https://api.example.com/users");
        assert_eq!(server, "https://api.example.com");
        assert_eq!(path, "/users");
        assert!(query.is_empty());
    }

    #[test]
    fn test_split_url_with_query() {
        let (server, path, query) = split_url("https://api.example.com/users?page=1&limit=10");
        assert_eq!(server, "https://api.example.com");
        assert_eq!(path, "/users");
        assert_eq!(query.len(), 2);
        assert!(query.iter().any(|(k, _)| k == "page"));
    }

    #[test]
    fn test_split_url_template_params() {
        let (_, path, _) = split_url("https://api.example.com/users/{{id}}/posts");
        assert_eq!(path, "/users/{id}/posts");
    }

    #[test]
    fn test_export_openapi_produces_valid_json() {
        use std::io::Write;
        let tmp = std::env::temp_dir().join(format!("protide_oas_test_{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().subsec_nanos()));
        std::fs::create_dir_all(&tmp).unwrap();

        let http_file = tmp.join("get_users.http");
        std::fs::File::create(&http_file).unwrap()
            .write_all(b"# @name getUsers\nGET https://api.example.com/users\nAccept: application/json\n").unwrap();

        let result = export_openapi(&tmp).unwrap();
        let _ = std::fs::remove_dir_all(&tmp);

        let parsed: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["openapi"].as_str(), Some("3.0.0"));
        assert!(parsed["paths"].get("/users").is_some());
        assert!(parsed["paths"]["/users"].get("get").is_some());
    }
}
