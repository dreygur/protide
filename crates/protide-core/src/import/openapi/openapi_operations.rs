use std::collections::HashMap;

use http_parser::{HttpMethod, KeyValue, Request, RequestMeta};
use serde_json::Value;

use super::openapi_security::{apply_security, SecuritySchemeInfo};
use super::schema::{get_example_value, get_schema_example};
use super::schema;

#[allow(clippy::too_many_arguments)]
pub(super) fn parse_operation(
    path: &str,
    method_str: &str,
    operation: &Value,
    base_url: &str,
    root: &Value,
    path_params: &[Value],
    security_schemes: &HashMap<String, SecuritySchemeInfo>,
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
            schema::resolve_ref(root, r).unwrap_or(param)
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
                schema::resolve_ref(root, r).unwrap_or(body_param)
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
            if let Some(schema_val) = resolved.get("schema") {
                let content_entry = serde_json::json!({ "schema": schema_val });
                body = get_schema_example(&content_entry, root);
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

pub(super) fn parse_request_body(body: &Value, headers: &mut Vec<KeyValue>, root: &Value) -> Option<String> {
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
