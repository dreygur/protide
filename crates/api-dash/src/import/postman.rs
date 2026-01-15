//! Postman Collection v2.1 parser
//!
//! Parses Postman Collection format into HTTP requests.

use http_parser::{HttpMethod, KeyValue, Request, RequestMeta, Scripts};
use serde::Deserialize;

use super::ImportResult;

/// Parse a Postman Collection JSON
pub fn parse_postman(input: &str) -> Result<ImportResult, String> {
    let collection: PostmanCollection = serde_json::from_str(input)
        .map_err(|e| format!("Failed to parse Postman collection: {}", e))?;

    let mut result = ImportResult::new();

    // Set collection name
    if let Some(info) = &collection.info {
        result.name = info.name.clone();
    }

    // Parse items recursively
    if let Some(items) = &collection.item {
        parse_items(items, &mut result, &[]);
    }

    Ok(result)
}

/// Recursively parse collection items (handles folders)
fn parse_items(items: &[PostmanItem], result: &mut ImportResult, path: &[String]) {
    for item in items {
        // Check if it's a folder (has nested items)
        if let Some(nested) = &item.item {
            let mut new_path = path.to_vec();
            if let Some(name) = &item.name {
                new_path.push(name.clone());
            }
            parse_items(nested, result, &new_path);
            continue;
        }

        // Parse request
        if let Some(request) = &item.request {
            match parse_request(item, request, path) {
                Ok(req) => result.add_request(req),
                Err(e) => result.add_warning(e),
            }
        }
    }
}

/// Parse a single Postman request
fn parse_request(item: &PostmanItem, request: &PostmanRequest, path: &[String]) -> Result<Request, String> {
    // Get method
    let method = request.method.as_ref()
        .and_then(|m| HttpMethod::from_str(m))
        .unwrap_or(HttpMethod::Get);

    // Get URL
    let url = match &request.url {
        Some(PostmanUrl::String(s)) => s.clone(),
        Some(PostmanUrl::Object(obj)) => obj.raw.clone().unwrap_or_default(),
        None => return Err("Request has no URL".to_string()),
    };

    // Build name from path and item name
    let name = if path.is_empty() {
        item.name.clone()
    } else {
        let prefix = path.join("/");
        item.name.as_ref().map(|n| format!("{}/{}", prefix, n))
    };

    // Parse headers
    let headers: Vec<KeyValue> = request.header.as_ref()
        .map(|headers| {
            headers.iter()
                .map(|h| {
                    let mut kv = KeyValue::new(
                        h.key.clone().unwrap_or_default(),
                        h.value.clone().unwrap_or_default(),
                    );
                    kv.enabled = !h.disabled.unwrap_or(false);
                    kv
                })
                .collect()
        })
        .unwrap_or_default();

    // Parse body
    let body = request.body.as_ref().and_then(|b| {
        match b.mode.as_deref() {
            Some("raw") => b.raw.clone(),
            Some("urlencoded") => {
                b.urlencoded.as_ref().map(|params| {
                    params.iter()
                        .filter(|p| !p.disabled.unwrap_or(false))
                        .map(|p| {
                            format!("{}={}",
                                urlencoding::encode(p.key.as_deref().unwrap_or("")),
                                urlencoding::encode(p.value.as_deref().unwrap_or("")),
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("&")
                })
            }
            Some("formdata") => {
                // Form data - just serialize as JSON-like for now
                b.formdata.as_ref().map(|params| {
                    let pairs: Vec<String> = params.iter()
                        .filter(|p| !p.disabled.unwrap_or(false))
                        .map(|p| {
                            format!("{}: {}",
                                p.key.as_deref().unwrap_or(""),
                                p.value.as_deref().unwrap_or(""),
                            )
                        })
                        .collect();
                    pairs.join("\n")
                })
            }
            Some("graphql") => {
                b.graphql.as_ref().map(|g| {
                    serde_json::json!({
                        "query": g.query,
                        "variables": g.variables
                    }).to_string()
                })
            }
            _ => None,
        }
    });

    // Parse scripts
    let mut scripts = Scripts::default();

    // Pre-request script
    if let Some(events) = &item.event {
        for event in events {
            if event.listen.as_deref() == Some("prerequest") {
                if let Some(script) = &event.script {
                    if let Some(exec) = &script.exec {
                        scripts.pre_script = Some(exec.join("\n"));
                    }
                }
            } else if event.listen.as_deref() == Some("test") {
                if let Some(script) = &event.script {
                    if let Some(exec) = &script.exec {
                        scripts.tests = Some(exec.join("\n"));
                    }
                }
            }
        }
    }

    let mut req = Request::new(method, url);
    req.headers = headers;
    req.body = body;
    req.scripts = scripts;
    req.meta = RequestMeta {
        name,
        description: request.description.clone().map(|d| match d {
            PostmanDescription::String(s) => s,
            PostmanDescription::Object { content, .. } => content.unwrap_or_default(),
        }),
        ..Default::default()
    };

    Ok(req)
}

// Postman Collection v2.1 types

#[derive(Debug, Deserialize)]
struct PostmanCollection {
    info: Option<PostmanInfo>,
    item: Option<Vec<PostmanItem>>,
}

#[derive(Debug, Deserialize)]
struct PostmanInfo {
    name: Option<String>,
    #[allow(dead_code)]
    schema: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PostmanItem {
    name: Option<String>,
    request: Option<PostmanRequest>,
    item: Option<Vec<PostmanItem>>, // Nested items (folder)
    event: Option<Vec<PostmanEvent>>,
}

#[derive(Debug, Deserialize)]
struct PostmanRequest {
    method: Option<String>,
    url: Option<PostmanUrl>,
    header: Option<Vec<PostmanHeader>>,
    body: Option<PostmanBody>,
    description: Option<PostmanDescription>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum PostmanUrl {
    String(String),
    Object(PostmanUrlObject),
}

#[derive(Debug, Deserialize)]
struct PostmanUrlObject {
    raw: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PostmanHeader {
    key: Option<String>,
    value: Option<String>,
    disabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct PostmanBody {
    mode: Option<String>,
    raw: Option<String>,
    urlencoded: Option<Vec<PostmanKeyValue>>,
    formdata: Option<Vec<PostmanKeyValue>>,
    graphql: Option<PostmanGraphQL>,
}

#[derive(Debug, Deserialize)]
struct PostmanKeyValue {
    key: Option<String>,
    value: Option<String>,
    disabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct PostmanGraphQL {
    query: Option<String>,
    variables: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum PostmanDescription {
    String(String),
    Object {
        content: Option<String>,
        #[serde(rename = "type")]
        #[allow(dead_code)]
        content_type: Option<String>,
    },
}

#[derive(Debug, Deserialize)]
struct PostmanEvent {
    listen: Option<String>,
    script: Option<PostmanScript>,
}

#[derive(Debug, Deserialize)]
struct PostmanScript {
    exec: Option<Vec<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_collection() {
        let json = r#"{
            "info": {
                "name": "Test Collection",
                "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
            },
            "item": [
                {
                    "name": "Get Users",
                    "request": {
                        "method": "GET",
                        "url": "https://api.example.com/users"
                    }
                }
            ]
        }"#;

        let result = parse_postman(json).unwrap();
        assert_eq!(result.name, Some("Test Collection".to_string()));
        assert_eq!(result.requests.len(), 1);
        assert_eq!(result.requests[0].method, HttpMethod::Get);
        assert_eq!(result.requests[0].url, "https://api.example.com/users");
    }

    #[test]
    fn test_request_with_headers() {
        let json = r#"{
            "info": {"name": "Test"},
            "item": [{
                "name": "Auth Request",
                "request": {
                    "method": "POST",
                    "url": "https://api.example.com/login",
                    "header": [
                        {"key": "Content-Type", "value": "application/json"},
                        {"key": "X-Api-Key", "value": "secret", "disabled": true}
                    ],
                    "body": {
                        "mode": "raw",
                        "raw": "{\"username\": \"test\"}"
                    }
                }
            }]
        }"#;

        let result = parse_postman(json).unwrap();
        let req = &result.requests[0];
        assert_eq!(req.method, HttpMethod::Post);
        assert_eq!(req.headers.len(), 2);
        assert!(req.headers[0].enabled);
        assert!(!req.headers[1].enabled);
        assert_eq!(req.body, Some(r#"{"username": "test"}"#.to_string()));
    }

    #[test]
    fn test_nested_folders() {
        let json = r#"{
            "info": {"name": "Test"},
            "item": [{
                "name": "Users",
                "item": [{
                    "name": "Get User",
                    "request": {
                        "method": "GET",
                        "url": "https://api.example.com/users/1"
                    }
                }]
            }]
        }"#;

        let result = parse_postman(json).unwrap();
        assert_eq!(result.requests.len(), 1);
        assert_eq!(result.requests[0].meta.name, Some("Users/Get User".to_string()));
    }
}
