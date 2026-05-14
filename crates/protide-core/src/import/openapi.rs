//! OpenAPI/Swagger specification parser
//!
//! Parses OpenAPI 3.x and Swagger 2.0 specifications into HTTP requests.

use http_parser::{HttpMethod, KeyValue, Request, RequestMeta};
use serde::Deserialize;
use serde_json::Value;

use super::ImportResult;

/// Parse an OpenAPI/Swagger specification
pub fn parse_openapi(input: &str) -> Result<ImportResult, String> {
    // Try to parse as JSON first
    let spec: OpenApiSpec = if input.trim().starts_with('{') {
        serde_json::from_str(input)
            .map_err(|e| format!("Failed to parse OpenAPI JSON: {}", e))?
    } else {
        // Try YAML
        serde_yaml::from_str(input)
            .map_err(|e| format!("Failed to parse OpenAPI YAML: {}", e))?
    };

    let mut result = ImportResult::new();

    // Set collection name from info
    if let Some(info) = &spec.info {
        result.name = info.title.clone();
    }

    // Get base URL
    let base_url = get_base_url(&spec);

    // Parse paths
    if let Some(paths) = &spec.paths {
        for (path, path_item) in paths {
            parse_path_item(path, path_item, &base_url, &mut result);
        }
    }

    Ok(result)
}

/// Get base URL from OpenAPI spec
fn get_base_url(spec: &OpenApiSpec) -> String {
    // OpenAPI 3.x - servers array
    if let Some(servers) = &spec.servers
        && let Some(first) = servers.first()
            && let Some(url) = &first.url {
                return url.clone();
            }

    // Swagger 2.0 - host, basePath, schemes
    let scheme = spec.schemes.as_ref()
        .and_then(|s| s.first())
        .cloned()
        .unwrap_or_else(|| "https".to_string());

    let host = spec.host.clone().unwrap_or_else(|| "localhost".to_string());
    let base_path = spec.base_path.clone().unwrap_or_default();

    format!("{}://{}{}", scheme, host, base_path)
}

/// Parse a path item (all operations for a path)
fn parse_path_item(path: &str, item: &Value, base_url: &str, result: &mut ImportResult) {
    let methods = ["get", "post", "put", "patch", "delete", "head", "options"];

    for method_str in methods {
        if let Some(operation) = item.get(method_str)
            && let Some(request) = parse_operation(path, method_str, operation, base_url) {
                result.add_request(request);
            }
    }
}

/// Parse a single operation
fn parse_operation(path: &str, method_str: &str, operation: &Value, base_url: &str) -> Option<Request> {
    let method = HttpMethod::from_str(method_str)?;

    // Build URL
    let url = format!("{}{}", base_url.trim_end_matches('/'), path);

    // Get operation ID or summary for name
    let name = operation.get("operationId")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            operation.get("summary")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        });

    let description = operation.get("description")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Parse parameters into headers and query params
    let mut headers: Vec<KeyValue> = Vec::new();

    if let Some(params) = operation.get("parameters").and_then(|v| v.as_array()) {
        for param in params {
            let param_in = param.get("in").and_then(|v| v.as_str()).unwrap_or("");
            let param_name = param.get("name").and_then(|v| v.as_str()).unwrap_or("");

            if param_in == "header" {
                // Add as header with placeholder value
                let example = get_example_value(param);
                headers.push(KeyValue::new(param_name, example));
            }
        }
    }

    // Add Content-Type header for request body
    let body = if let Some(request_body) = operation.get("requestBody") {
        parse_request_body(request_body, &mut headers)
    } else {
        None
    };

    // Add Accept header based on produces/responses
    if let Some(produces) = operation.get("produces").and_then(|v| v.as_array())
        && let Some(first) = produces.first().and_then(|v| v.as_str()) {
            headers.push(KeyValue::new("Accept", first));
        }

    let mut request = Request::new(method, url);
    request.headers = headers;
    request.body = body;
    request.meta = RequestMeta {
        name,
        description,
        ..Default::default()
    };

    Some(request)
}

/// Parse request body from OpenAPI 3.x
fn parse_request_body(body: &Value, headers: &mut Vec<KeyValue>) -> Option<String> {
    let content = body.get("content")?;

    // Prefer JSON
    if let Some(json_content) = content.get("application/json") {
        headers.push(KeyValue::new("Content-Type", "application/json"));
        return get_schema_example(json_content);
    }

    // Try form data
    if let Some(form_content) = content.get("application/x-www-form-urlencoded") {
        headers.push(KeyValue::new("Content-Type", "application/x-www-form-urlencoded"));
        return get_schema_example(form_content);
    }

    // Try multipart
    if let Some(multipart_content) = content.get("multipart/form-data") {
        headers.push(KeyValue::new("Content-Type", "multipart/form-data"));
        return get_schema_example(multipart_content);
    }

    // Use first available content type
    if let Some(obj) = content.as_object()
        && let Some((content_type, type_content)) = obj.iter().next() {
            headers.push(KeyValue::new("Content-Type", content_type.clone()));
            return get_schema_example(type_content);
        }

    None
}

/// Get example value from schema
fn get_schema_example(content: &Value) -> Option<String> {
    // Check for direct example
    if let Some(example) = content.get("example") {
        return Some(if example.is_string() {
            example.as_str().unwrap().to_string()
        } else {
            serde_json::to_string_pretty(example).ok()?
        });
    }

    // Check schema for example
    if let Some(schema) = content.get("schema") {
        if let Some(example) = schema.get("example") {
            return Some(if example.is_string() {
                example.as_str().unwrap().to_string()
            } else {
                serde_json::to_string_pretty(example).ok()?
            });
        }

        // Generate example from schema type
        return generate_schema_example(schema);
    }

    None
}

/// Generate example value from schema definition
fn generate_schema_example(schema: &Value) -> Option<String> {
    let schema_type = schema.get("type").and_then(|v| v.as_str())?;

    match schema_type {
        "object" => {
            let mut obj = serde_json::Map::new();

            if let Some(properties) = schema.get("properties").and_then(|v| v.as_object()) {
                for (key, prop_schema) in properties {
                    let value = generate_property_example(prop_schema);
                    obj.insert(key.clone(), value);
                }
            }

            serde_json::to_string_pretty(&Value::Object(obj)).ok()
        }
        "array" => {
            if let Some(items) = schema.get("items") {
                let item_example = generate_property_example(items);
                serde_json::to_string_pretty(&Value::Array(vec![item_example])).ok()
            } else {
                Some("[]".to_string())
            }
        }
        "string" => Some("\"string\"".to_string()),
        "integer" | "number" => Some("0".to_string()),
        "boolean" => Some("true".to_string()),
        _ => None,
    }
}

/// Generate example value for a property
fn generate_property_example(schema: &Value) -> Value {
    // Check for example first
    if let Some(example) = schema.get("example") {
        return example.clone();
    }

    let schema_type = schema.get("type").and_then(|v| v.as_str()).unwrap_or("string");

    match schema_type {
        "string" => {
            // Check for format
            match schema.get("format").and_then(|v| v.as_str()) {
                Some("email") => Value::String("user@example.com".to_string()),
                Some("uri") | Some("url") => Value::String("https://example.com".to_string()),
                Some("uuid") => Value::String("550e8400-e29b-41d4-a716-446655440000".to_string()),
                Some("date") => Value::String("2024-01-01".to_string()),
                Some("date-time") => Value::String("2024-01-01T00:00:00Z".to_string()),
                _ => Value::String("string".to_string()),
            }
        }
        "integer" => Value::Number(0.into()),
        "number" => serde_json::Number::from_f64(0.0).map(Value::Number).unwrap_or(Value::Null),
        "boolean" => Value::Bool(true),
        "array" => {
            if let Some(items) = schema.get("items") {
                Value::Array(vec![generate_property_example(items)])
            } else {
                Value::Array(vec![])
            }
        }
        "object" => {
            let mut obj = serde_json::Map::new();
            if let Some(properties) = schema.get("properties").and_then(|v| v.as_object()) {
                for (key, prop_schema) in properties {
                    obj.insert(key.clone(), generate_property_example(prop_schema));
                }
            }
            Value::Object(obj)
        }
        _ => Value::Null,
    }
}

/// Get example value from parameter
fn get_example_value(param: &Value) -> String {
    // Check for example
    if let Some(example) = param.get("example") {
        if let Some(s) = example.as_str() {
            return s.to_string();
        }
        return example.to_string();
    }

    // Check schema for example
    if let Some(schema) = param.get("schema") {
        if let Some(example) = schema.get("example") {
            if let Some(s) = example.as_str() {
                return s.to_string();
            }
            return example.to_string();
        }

        // Use default based on type
        let schema_type = schema.get("type").and_then(|v| v.as_str()).unwrap_or("string");
        return match schema_type {
            "string" => "{{value}}".to_string(),
            "integer" | "number" => "0".to_string(),
            "boolean" => "true".to_string(),
            _ => "{{value}}".to_string(),
        };
    }

    "{{value}}".to_string()
}

// OpenAPI/Swagger spec types

#[derive(Debug, Deserialize)]
struct OpenApiSpec {
    info: Option<OpenApiInfo>,
    // OpenAPI 3.x
    servers: Option<Vec<OpenApiServer>>,
    // Swagger 2.0
    host: Option<String>,
    #[serde(rename = "basePath")]
    base_path: Option<String>,
    schemes: Option<Vec<String>>,
    // Paths (common)
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
}
