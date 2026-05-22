//! OpenAPI/Swagger specification parser
//!
//! Parses OpenAPI 3.x and Swagger 2.0 specifications into HTTP requests.

mod openapi_operations;
mod openapi_paths;
mod openapi_security;
mod schema;

use serde::Deserialize;
use serde_json::Value;

use super::ImportResult;
use openapi_paths::parse_path_item;
use openapi_security::extract_security_schemes;

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
    use http_parser::HttpMethod;

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
