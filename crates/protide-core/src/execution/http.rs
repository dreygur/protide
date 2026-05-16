use std::time::{Duration, Instant};

use super::{ExecutionBody, ExecutionMode, FormPartValue};

/// Raw HTTP response before scripting/extraction
pub struct RawResponse {
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub time: Duration,
    pub size: usize,
}

/// Execute a blocking HTTP (or GraphQL-over-HTTP) request.
pub fn run_http(
    url: &str,
    method: &str,
    headers: &[(String, String)],
    body: &ExecutionBody,
    mode: &ExecutionMode,
    client_cert: Option<(&std::path::Path, &std::path::Path)>,
) -> Result<RawResponse, String> {
    let start = Instant::now();

    // GraphQL: construct JSON body and ensure Content-Type
    let (resolved_url, resolved_headers, resolved_body) = match mode {
        ExecutionMode::GraphQL { query, variables, operation_name } => {
            let vars: serde_json::Value = if variables.trim().is_empty() {
                serde_json::json!({})
            } else {
                serde_json::from_str(variables)
                    .map_err(|e| format!("GraphQL variables are not valid JSON: {e}"))?
            };
            let mut gql_body = serde_json::json!({
                "query": query,
                "variables": vars,
            });
            if let Some(op) = operation_name
                && !op.is_empty() {
                    gql_body["operationName"] = serde_json::Value::String(op.clone());
                }
            let mut hdrs = headers.to_vec();
            if !hdrs.iter().any(|(k, _)| k.eq_ignore_ascii_case("content-type")) {
                hdrs.push(("Content-Type".to_string(), "application/json".to_string()));
            }
            (url.to_string(), hdrs, ExecutionBody::Text(gql_body.to_string()))
        }
        ExecutionMode::Http => (url.to_string(), headers.to_vec(), body.clone()),
    };

    let req_method = reqwest::Method::from_bytes(method.as_bytes())
        .unwrap_or(reqwest::Method::GET);

    let mut client_builder = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30));
    if let Some((cert_path, key_path)) = client_cert {
        let cert_pem = std::fs::read(cert_path).map_err(|e| format!("mTLS cert read error: {e}"))?;
        let key_pem  = std::fs::read(key_path).map_err(|e| format!("mTLS key read error: {e}"))?;
        // from_pem expects a single buffer with both key and cert (rustls-tls)
        let mut combined = key_pem;
        combined.extend_from_slice(&cert_pem);
        let identity = reqwest::Identity::from_pem(&combined)
            .map_err(|e| format!("mTLS identity error: {e}"))?;
        client_builder = client_builder.identity(identity);
    }
    let client = client_builder.build().map_err(|e| e.to_string())?;
    let mut req_builder = client.request(req_method, &resolved_url);

    let is_multipart = matches!(resolved_body, ExecutionBody::Multipart(_));
    for (key, value) in &resolved_headers {
        // Let reqwest set Content-Type with boundary for multipart
        if is_multipart && key.eq_ignore_ascii_case("content-type") {
            continue;
        }
        req_builder = req_builder.header(key.as_str(), value.as_str());
    }

    match &resolved_body {
        ExecutionBody::None => {}
        ExecutionBody::Text(s) => {
            req_builder = req_builder.body(s.clone());
        }
        ExecutionBody::Binary(bytes) => {
            req_builder = req_builder.body(bytes.clone());
        }
        ExecutionBody::Multipart(parts) => {
            let mut form = reqwest::blocking::multipart::Form::new();
            for part in parts {
                match &part.value {
                    FormPartValue::Text(v) => {
                        form = form.text(part.name.clone(), v.clone());
                    }
                    FormPartValue::File(path) => {
                        if let Ok(p) = reqwest::blocking::multipart::Part::file(path) {
                            form = form.part(part.name.clone(), p);
                        }
                    }
                }
            }
            req_builder = req_builder.multipart(form);
        }
    }

    let response = req_builder.send().map_err(|e| e.to_string())?;
    let elapsed = start.elapsed();

    let status = response.status().as_u16();
    let status_text = status_text(status).to_string();
    let resp_headers: Vec<(String, String)> = response
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();
    let body_str = response.text().unwrap_or_default();
    let size = body_str.len();

    Ok(RawResponse {
        status,
        status_text,
        headers: resp_headers,
        body: body_str,
        time: elapsed,
        size,
    })
}

fn status_text(status: u16) -> &'static str {
    match status {
        100 => "Continue",
        101 => "Switching Protocols",
        200 => "OK",
        201 => "Created",
        202 => "Accepted",
        204 => "No Content",
        301 => "Moved Permanently",
        302 => "Found",
        304 => "Not Modified",
        307 => "Temporary Redirect",
        308 => "Permanent Redirect",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        408 => "Request Timeout",
        409 => "Conflict",
        422 => "Unprocessable Entity",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        _ => "Unknown",
    }
}
