use std::collections::HashMap;

use http_parser::KeyValue;
use serde_json::Value;

#[derive(Debug, Clone)]
pub(super) enum SecuritySchemeInfo {
    BearerHttp,
    BasicHttp,
    ApiKeyHeader(String),
    ApiKeyQuery(String),
    OAuth2,
}

pub(super) fn parse_security_scheme(def: &Value) -> Option<SecuritySchemeInfo> {
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

/// Extract all security schemes from root (OAS 3 components + Swagger 2 securityDefinitions)
pub(super) fn extract_security_schemes(root: &Value) -> HashMap<String, SecuritySchemeInfo> {
    let mut schemes = HashMap::new();

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

pub(super) fn apply_security(
    security: &[Value],
    schemes: &HashMap<String, SecuritySchemeInfo>,
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
