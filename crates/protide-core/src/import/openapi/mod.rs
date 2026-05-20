//! OpenAPI/Swagger specification parser
//!
//! Parses OpenAPI 3.x and Swagger 2.0 specifications into HTTP requests.

mod schema;

use http_parser::{HttpMethod, KeyValue, Request, RequestMeta};
use serde::Deserialize;
use serde_json::Value;

use super::ImportResult;
use schema::{get_example_value, get_schema_example, resolve_ref};

/// Parse an OpenAPI/Swagger specification
pub fn parse_openapi(input: &str) -> Result<ImportResult, String> {
    let root: Value = if input.trim().starts_with('{') {
        serde_json::from_str(input)
            .map_err(|e| format!("Failed to parse OpenAPI JSON: {}", e))?
    } else {
        serde_yaml::from_str(input)
            .map_err(|e| format!("Failed to parse OpenAPI YAML: {}", e))?
    };

    let spec: OpenApiSpec = serde_json::from_value(root.clone())
        .map_err(|e| format!("Failed to interpret OpenAPI spec: {}", e))?;

    let mut result = ImportResult::new();

    if let Some(info) = &spec.info {
        result.name = info.title.clone();
    }

    let base_url = get_base_url(&spec);
    let security_schemes = extract_security_schemes(&root);
    let global_consumes: Vec<String> = root.get("consumes")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default();
    let global_security = root.get("security")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter()
            .filter_map(|s| s.as_object())
            .filter_map(|obj| obj.keys().next().cloned())
            .collect::<Vec<_>>())
        .unwrap_or_default();

    if let Some(paths) = &spec.paths {
        for (path, path_item) in paths {
            parse_path_item(
                path,
                path_item,
                &base_url,
                &root,
                &security_schemes,
                &global_consumes,
                &global_security,
                &mut result,
            );
        }
    }

    Ok(result)
}

fn get_base_url(spec: &OpenApiSpec) -> String {
    if let Some(servers) = &spec.servers
        && let Some(first) = servers.first()
            && let Some(url) = &first.url {
                return url.clone();
            }

    let scheme = spec.schemes.as_ref()
        .and_then(|s| s.first())
        .cloned()
        .unwrap_or_else(|| "https".to_string());

    let host = spec.host.clone().unwrap_or_else(|| "localhost".to_string());
    let base_path = spec.base_path.clone().unwrap_or_default();

    format!("{}://{}{}", scheme, host, base_path)
}

/// Extract all security schemes from root (OAS 3 components + Swagger 2 securityDefinitions)
fn extract_security_schemes(root: &Value) -> std::collections::HashMap<String, SecuritySchemeInfo> {
    let mut schemes = std::collections::HashMap::new();

    // OAS 3: components.securitySchemes
    if let Some(defs) = root
        .get("components")
        .and_then(|c| c.get("securitySchemes"))
        .and_then(|v| v.as_object())
    {
        for (name, def) in defs {
            if let Some(info) = parse_security_scheme(def) {
                schemes.insert(name.clone(), info);
            }
        }
    }

    // Swagger 2: securityDefinitions
    if let Some(defs) = root.get("securityDefinitions").and_then(|v| v.as_object()) {
        for (name, def) in defs {
            if let Some(info) = parse_security_scheme(def) {
                schemes.insert(name.clone(), info);
            }
        }
    }

    schemes
}

#[derive(Debug, Clone)]
enum SecuritySchemeInfo {
    BearerHttp,
    BasicHttp,
    ApiKeyHeader(String),
    ApiKeyQuery(String),
    OAuth2,
}

fn parse_security_scheme(def: &Value) -> Option<SecuritySchemeInfo> {
    let scheme_type = def.get("type").and_then(|v| v.as_str()).unwrap_or("");
    match scheme_type {
        "http" => {
            match def.get("scheme").and_then(|v| v.as_str()).unwrap_or("") {
                "bearer" => Some(SecuritySchemeInfo::BearerHttp),
                "basic" => Some(SecuritySchemeInfo::BasicHttp),
                _ => None,
            }
        }
        "apiKey" => {
            let name = def.get("name").and_then(|v| v.as_str())?.to_string();
            match def.get("in").and_then(|v| v.as_str()).unwrap_or("") {
                "header" => Some(SecuritySchemeInfo::ApiKeyHeader(name)),
                "query" => Some(SecuritySchemeInfo::ApiKeyQuery(name)),
                _ => None,
            }
        }
        "oauth2" => Some(SecuritySchemeInfo::OAuth2),
        // Swagger 2 apiKey
        "apikey" => {
            let name = def.get("name").and_then(|v| v.as_str())?.to_string();
            match def.get("in").and_then(|v| v.as_str()).unwrap_or("") {
                "header" => Some(SecuritySchemeInfo::ApiKeyHeader(name)),
                "query" => Some(SecuritySchemeInfo::ApiKeyQuery(name)),
                _ => None,
            }
        }
        _ => None,
    }
}

fn apply_security(
    security: &[Value],
    schemes: &std::collections::HashMap<String, SecuritySchemeInfo>,
    headers: &mut Vec<KeyValue>,
    query_parts: &mut Vec<String>,
) {
    for req in security {
        if let Some(obj) = req.as_object() {
            if let Some(scheme_name) = obj.keys().next() {
                if let Some(info) = schemes.get(scheme_name) {
                    match info {
                        SecuritySchemeInfo::BearerHttp | SecuritySchemeInfo::OAuth2 => {
                            if !headers.iter().any(|h| h.key.eq_ignore_ascii_case("Authorization")) {
                                headers.push(KeyValue::new("Authorization", "Bearer {{token}}"));
                            }
                        }
                        SecuritySchemeInfo::BasicHttp => {
                            if !headers.iter().any(|h| h.key.eq_ignore_ascii_case("Authorization")) {
                                headers.push(KeyValue::new("Authorization", "Basic {{credentials}}"));
                            }
                        }
                        SecuritySchemeInfo::ApiKeyHeader(name) => {
                            if !headers.iter().any(|h| h.key == name.as_str()) {
                                headers.push(KeyValue::new(name.clone(), "{{api_key}}"));
                            }
                        }
                        SecuritySchemeInfo::ApiKeyQuery(name) => {
                            if !query_parts.iter().any(|q| q.starts_with(name.as_str())) {
                                query_parts.push(format!("{}={{{{api_key}}}}", name));
                            }
                        }
                    }
                    break;
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn parse_path_item(
    path: &str,
    item: &Value,
    base_url: &str,
    root: &Value,
    security_schemes: &std::collections::HashMap<String, SecuritySchemeInfo>,
    global_consumes: &[String],
    global_security: &[String],
    result: &mut ImportResult,
) {
    let methods = ["get", "post", "put", "patch", "delete", "head", "options"];
    let path_params = item.get("parameters").and_then(|v| v.as_array()).cloned().unwrap_or_default();

    for method_str in methods {
        if let Some(operation) = item.get(method_str) {
            if let Some((folder, req)) = parse_operation(
                path,
                method_str,
                operation,
                base_url,
                root,
                &path_params,
                security_schemes,
                global_consumes,
                global_security,
            ) {
                result.add_request_in_folder(req, folder);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn parse_operation(
    path: &str,
    method_str: &str,
    operation: &Value,
    base_url: &str,
    root: &Value,
    path_params: &[Value],
    security_schemes: &std::collections::HashMap<String, SecuritySchemeInfo>,
    global_consumes: &[String],
    global_security: &[String],
) -> Option<(Option<String>, Request)> {
    let method = HttpMethod::from_str(method_str)?;

    // Convert OpenAPI path params {id} → protide env vars {{id}}
    let normalized_path = path.replace('{', "{{").replace('}', "}}");

    let name = operation.get("operationId")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| operation.get("summary").and_then(|v| v.as_str()).map(|s| s.to_string()));

    let description = operation.get("description").and_then(|v| v.as_str()).map(|s| s.to_string());

    // First tag → folder grouping
    let folder = operation.get("tags")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let mut headers: Vec<KeyValue> = Vec::new();
    let mut query_parts: Vec<String> = Vec::new();

    // Merge path-level params with operation-level (operation overrides path-level)
    let op_params = operation.get("parameters").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let all_params: Vec<&Value> = {
        let mut merged: Vec<&Value> = path_params.iter().collect();
        for op_p in &op_params {
            let op_name = op_p.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let op_in = op_p.get("in").and_then(|v| v.as_str()).unwrap_or("");
            merged.retain(|p| {
                !(p.get("name").and_then(|v| v.as_str()).unwrap_or("") == op_name
                    && p.get("in").and_then(|v| v.as_str()).unwrap_or("") == op_in)
            });
            merged.push(op_p);
        }
        merged
    };

    for param in &all_params {
        let resolved: &Value = if let Some(r) = param.get("$ref").and_then(|v| v.as_str()) {
            resolve_ref(root, r).unwrap_or(param)
        } else {
            param
        };

        let param_in = resolved.get("in").and_then(|v| v.as_str()).unwrap_or("");
        let param_name = resolved.get("name").and_then(|v| v.as_str()).unwrap_or("");

        match param_in {
            "header" => headers.push(KeyValue::new(param_name, get_example_value(resolved, root))),
            "query" => query_parts.push(format!("{}={}", param_name, get_example_value(resolved, root))),
            _ => {}
        }
    }

    let mut body = if let Some(request_body) = operation.get("requestBody") {
        parse_request_body(request_body, &mut headers, root)
    } else {
        None
    };

    // Swagger 2.0: body parameter + consumes-driven Content-Type
    if body.is_none() {
        if let Some(body_param) = all_params.iter().find(|p| {
            p.get("in").and_then(|v| v.as_str()) == Some("body")
        }) {
            let resolved = if let Some(r) = body_param.get("$ref").and_then(|v| v.as_str()) {
                resolve_ref(root, r).unwrap_or(body_param)
            } else {
                body_param
            };
            let op_consumes: Vec<String> = operation.get("consumes")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();
            let content_type = op_consumes.first()
                .or_else(|| global_consumes.first())
                .map(|s| s.as_str())
                .unwrap_or("application/json");
            if !headers.iter().any(|h| h.key.eq_ignore_ascii_case("Content-Type")) {
                headers.push(KeyValue::new("Content-Type", content_type));
            }
            if let Some(schema) = resolved.get("schema") {
                let content_entry = serde_json::json!({ "schema": schema });
                body = schema::get_schema_example(&content_entry, root);
            }
        }
    }

    // Swagger 2.0 Accept from produces
    if let Some(produces) = operation.get("produces").and_then(|v| v.as_array()) {
        if let Some(first) = produces.first().and_then(|v| v.as_str()) {
            if !headers.iter().any(|h| h.key.eq_ignore_ascii_case("Accept")) {
                headers.push(KeyValue::new("Accept", first));
            }
        }
    }

    // Security: operation-level then global
    let op_security: Vec<Value> = operation.get("security")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    if !op_security.is_empty() {
        apply_security(&op_security, security_schemes, &mut headers, &mut query_parts);
    } else if !global_security.is_empty() {
        let fallback: Vec<Value> = global_security.iter()
            .map(|name| serde_json::json!({ name: [] }))
            .collect();
        apply_security(&fallback, security_schemes, &mut headers, &mut query_parts);
    }

    let url = if query_parts.is_empty() {
        format!("{}{}", base_url.trim_end_matches('/'), normalized_path)
    } else {
        format!("{}{}?{}", base_url.trim_end_matches('/'), normalized_path, query_parts.join("&"))
    };

    let mut request = Request::new(method, url);
    request.headers = headers;
    request.body = body;
    request.meta = RequestMeta { name, description, ..Default::default() };

    Some((folder, request))
}

fn parse_request_body(body: &Value, headers: &mut Vec<KeyValue>, root: &Value) -> Option<String> {
    let content = body.get("content")?;

    if let Some(json_content) = content.get("application/json") {
        if !headers.iter().any(|h| h.key.eq_ignore_ascii_case("Content-Type")) {
            headers.push(KeyValue::new("Content-Type", "application/json"));
        }
        return get_schema_example(json_content, root);
    }
    if let Some(form_content) = content.get("application/x-www-form-urlencoded") {
        if !headers.iter().any(|h| h.key.eq_ignore_ascii_case("Content-Type")) {
            headers.push(KeyValue::new("Content-Type", "application/x-www-form-urlencoded"));
        }
        return get_schema_example(form_content, root);
    }
    if let Some(mp_content) = content.get("multipart/form-data") {
        if !headers.iter().any(|h| h.key.eq_ignore_ascii_case("Content-Type")) {
            headers.push(KeyValue::new("Content-Type", "multipart/form-data"));
        }
        return get_schema_example(mp_content, root);
    }
    if let Some(obj) = content.as_object() {
        if let Some((ct, type_content)) = obj.iter().next() {
            if !headers.iter().any(|h| h.key.eq_ignore_ascii_case("Content-Type")) {
                headers.push(KeyValue::new("Content-Type", ct.clone()));
            }
            return get_schema_example(type_content, root);
        }
    }

    None
}

#[derive(Debug, Deserialize)]
struct OpenApiSpec {
    info: Option<OpenApiInfo>,
    servers: Option<Vec<OpenApiServer>>,
    host: Option<String>,
    #[serde(rename = "basePath")]
    base_path: Option<String>,
    schemes: Option<Vec<String>>,
    paths: Option<serde_json::Map<String, Value>>,
}

#[derive(Debug, Deserialize)]
struct OpenApiInfo {
    title: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenApiServer {
    url: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openapi3_simple() {
        let json = r#"{
            "openapi": "3.0.0",
            "info": {"title": "Test API"},
            "servers": [{"url": "https://api.example.com"}],
            "paths": {
                "/users": {
                    "get": {
                        "operationId": "getUsers",
                        "summary": "Get all users"
                    }
                }
            }
        }"#;

        let result = parse_openapi(json).unwrap();
        assert_eq!(result.name, Some("Test API".to_string()));
        assert_eq!(result.requests.len(), 1);
        assert_eq!(result.requests[0].method, HttpMethod::Get);
        assert_eq!(result.requests[0].url, "https://api.example.com/users");
        assert_eq!(result.requests[0].meta.name, Some("getUsers".to_string()));
    }

    #[test]
    fn test_swagger2_simple() {
        let json = r#"{
            "swagger": "2.0",
            "info": {"title": "Test API"},
            "host": "api.example.com",
            "basePath": "/v1",
            "schemes": ["https"],
            "paths": {
                "/users": {
                    "post": {
                        "operationId": "createUser"
                    }
                }
            }
        }"#;

        let result = parse_openapi(json).unwrap();
        assert_eq!(result.requests.len(), 1);
        assert_eq!(result.requests[0].method, HttpMethod::Post);
        assert_eq!(result.requests[0].url, "https://api.example.com/v1/users");
    }

    #[test]
    fn test_multiple_methods() {
        let json = r#"{
            "openapi": "3.0.0",
            "info": {"title": "Test"},
            "servers": [{"url": "https://api.example.com"}],
            "paths": {
                "/users/{id}": {
                    "get": {"operationId": "getUser"},
                    "put": {"operationId": "updateUser"},
                    "delete": {"operationId": "deleteUser"}
                }
            }
        }"#;

        let result = parse_openapi(json).unwrap();
        assert_eq!(result.requests.len(), 3);
    }

    #[test]
    fn test_request_body_example() {
        let json = r#"{
            "openapi": "3.0.0",
            "info": {"title": "Test"},
            "servers": [{"url": "https://api.example.com"}],
            "paths": {
                "/users": {
                    "post": {
                        "operationId": "createUser",
                        "requestBody": {
                            "content": {
                                "application/json": {
                                    "example": {"name": "John", "email": "john@example.com"}
                                }
                            }
                        }
                    }
                }
            }
        }"#;

        let result = parse_openapi(json).unwrap();
        assert_eq!(result.requests.len(), 1);
        assert!(result.requests[0].body.is_some());
        assert!(result.requests[0].headers.iter().any(|h| h.key == "Content-Type" && h.value == "application/json"));
    }

    #[test]
    fn test_path_params_converted() {
        let json = r#"{
            "openapi": "3.0.0",
            "info": {"title": "Test"},
            "servers": [{"url": "https://api.example.com"}],
            "paths": {
                "/users/{id}/posts/{postId}": {
                    "get": {"operationId": "getPost"}
                }
            }
        }"#;

        let result = parse_openapi(json).unwrap();
        assert_eq!(result.requests[0].url, "https://api.example.com/users/{{id}}/posts/{{postId}}");
    }

    #[test]
    fn test_query_params_appended_to_url() {
        let json = r#"{
            "openapi": "3.0.0",
            "info": {"title": "Test"},
            "servers": [{"url": "https://api.example.com"}],
            "paths": {
                "/users": {
                    "get": {
                        "operationId": "listUsers",
                        "parameters": [
                            {"in": "query", "name": "page", "schema": {"type": "integer"}},
                            {"in": "query", "name": "limit", "schema": {"type": "integer"}}
                        ]
                    }
                }
            }
        }"#;

        let result = parse_openapi(json).unwrap();
        let url = &result.requests[0].url;
        assert!(url.contains("page=0"), "page param missing: {}", url);
        assert!(url.contains("limit=0"), "limit param missing: {}", url);
    }

    #[test]
    fn test_ref_resolution_in_schema() {
        let json = r##"{
            "openapi": "3.0.0",
            "info": {"title": "Test"},
            "servers": [{"url": "https://api.example.com"}],
            "components": {
                "schemas": {
                    "User": {
                        "type": "object",
                        "properties": {
                            "id": {"type": "integer"},
                            "name": {"type": "string"}
                        }
                    }
                }
            },
            "paths": {
                "/users": {
                    "post": {
                        "operationId": "createUser",
                        "requestBody": {
                            "content": {
                                "application/json": {
                                    "schema": {"$ref": "#/components/schemas/User"}
                                }
                            }
                        }
                    }
                }
            }
        }"##;

        let result = parse_openapi(json).unwrap();
        let body = result.requests[0].body.as_ref().expect("body must exist for $ref schema");
        let parsed: serde_json::Value = serde_json::from_str(body).expect("body must be valid JSON");
        assert!(parsed.get("id").is_some(), "id property missing from generated body");
        assert!(parsed.get("name").is_some(), "name property missing from generated body");
    }

    #[test]
    fn test_path_level_parameters() {
        let json = r#"{
            "openapi": "3.0.0",
            "info": {"title": "Test"},
            "servers": [{"url": "https://api.example.com"}],
            "paths": {
                "/users": {
                    "parameters": [
                        {"in": "header", "name": "X-Tenant", "schema": {"type": "string"}}
                    ],
                    "get": {"operationId": "listUsers"},
                    "post": {"operationId": "createUser"}
                }
            }
        }"#;

        let result = parse_openapi(json).unwrap();
        assert_eq!(result.requests.len(), 2);
        for req in &result.requests {
            assert!(
                req.headers.iter().any(|h| h.key == "X-Tenant"),
                "path-level X-Tenant header missing from {}",
                req.meta.name.as_deref().unwrap_or("?")
            );
        }
    }

    #[test]
    fn test_tags_become_folder() {
        let json = r#"{
            "openapi": "3.0.0",
            "info": {"title": "Test"},
            "servers": [{"url": "https://api.example.com"}],
            "paths": {
                "/users": {
                    "get": {
                        "operationId": "listUsers",
                        "tags": ["users"]
                    },
                    "post": {
                        "operationId": "createUser",
                        "tags": ["users"]
                    }
                },
                "/posts": {
                    "get": {
                        "operationId": "listPosts",
                        "tags": ["posts"]
                    }
                }
            }
        }"#;

        let result = parse_openapi(json).unwrap();
        assert_eq!(result.requests.len(), 3);
        assert_eq!(result.request_folders.len(), 3);
        assert!(result.request_folders.iter().any(|f| f.as_deref() == Some("users")));
        assert!(result.request_folders.iter().any(|f| f.as_deref() == Some("posts")));
    }

    #[test]
    fn test_bearer_security_scheme() {
        let json = r#"{
            "openapi": "3.0.0",
            "info": {"title": "Test"},
            "servers": [{"url": "https://api.example.com"}],
            "components": {
                "securitySchemes": {
                    "bearerAuth": {
                        "type": "http",
                        "scheme": "bearer"
                    }
                }
            },
            "security": [{"bearerAuth": []}],
            "paths": {
                "/users": {
                    "get": {"operationId": "listUsers"}
                }
            }
        }"#;

        let result = parse_openapi(json).unwrap();
        assert!(
            result.requests[0].headers.iter().any(|h| h.key == "Authorization" && h.value.starts_with("Bearer")),
            "Bearer auth header missing"
        );
    }

    #[test]
    fn test_allof_schema() {
        let json = r##"{
            "openapi": "3.0.0",
            "info": {"title": "Test"},
            "servers": [{"url": "https://api.example.com"}],
            "components": {
                "schemas": {
                    "Base": {
                        "type": "object",
                        "properties": {
                            "id": {"type": "integer"}
                        }
                    }
                }
            },
            "paths": {
                "/items": {
                    "post": {
                        "operationId": "createItem",
                        "requestBody": {
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "allOf": [
                                            {"$ref": "#/components/schemas/Base"},
                                            {
                                                "type": "object",
                                                "properties": {
                                                    "name": {"type": "string"}
                                                }
                                            }
                                        ]
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }"##;

        let result = parse_openapi(json).unwrap();
        let body = result.requests[0].body.as_ref().expect("allOf body must exist");
        let parsed: serde_json::Value = serde_json::from_str(body).expect("body must be valid JSON");
        assert!(parsed.get("id").is_some(), "id from allOf Base missing");
        assert!(parsed.get("name").is_some(), "name from allOf inline missing");
    }
}
