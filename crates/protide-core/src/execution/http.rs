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

/// Resolve URL, headers, and body for the given execution mode.
/// GraphQL wraps the query into a JSON body and injects Content-Type.
fn resolve_request(
    url: &str,
    headers: &[(String, String)],
    body: &ExecutionBody,
    mode: &ExecutionMode,
) -> (String, Vec<(String, String)>, ExecutionBody) {
    match mode {
        ExecutionMode::GraphQL { query, variables, operation_name } => {
            let vars: serde_json::Value = serde_json::from_str(variables)
                .unwrap_or(serde_json::json!({}));
            let mut gql_body = serde_json::json!({
                "query": query,
                "variables": vars,
            });
            if let Some(op) = operation_name
                && !op.is_empty()
            {
                gql_body["operationName"] = serde_json::Value::String(op.clone());
            }
            let mut hdrs = headers.to_vec();
            if !hdrs.iter().any(|(k, _)| k.eq_ignore_ascii_case("content-type")) {
                hdrs.push(("Content-Type".to_string(), "application/json".to_string()));
            }
            (url.to_string(), hdrs, ExecutionBody::Text(gql_body.to_string()))
        }
        ExecutionMode::Http => (url.to_string(), headers.to_vec(), body.clone()),
    }
}

/// Chrome 131 browser header fingerprint (Windows, en-US).
/// Applied when `impersonate_browser` is true.  Existing user-supplied values
/// for the same header names are preserved (user headers take precedence).
const CHROME_PROFILE: &[(&str, &str)] = &[
    ("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36"),
    ("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7"),
    ("Accept-Language", "en-US,en;q=0.9"),
    ("Accept-Encoding", "gzip, deflate, br, zstd"),
    ("sec-ch-ua", "\"Google Chrome\";v=\"131\", \"Chromium\";v=\"131\", \"Not_A Brand\";v=\"24\""),
    ("sec-ch-ua-mobile", "?0"),
    ("sec-ch-ua-platform", "\"Windows\""),
    ("Upgrade-Insecure-Requests", "1"),
    ("sec-fetch-dest", "document"),
    ("sec-fetch-mode", "navigate"),
    ("sec-fetch-site", "none"),
    ("sec-fetch-user", "?1"),
];

/// Build the header list for a request with the Chrome browser profile prepended.
/// User-supplied headers override matching profile entries so explicit values win.
fn apply_browser_profile(user_headers: &[(String, String)]) -> Vec<(String, String)> {
    let mut result: Vec<(String, String)> = CHROME_PROFILE
        .iter()
        .filter(|(name, _)| {
            !user_headers
                .iter()
                .any(|(k, _)| k.eq_ignore_ascii_case(name))
        })
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect();
    result.extend_from_slice(user_headers);
    result
}

/// Execute a blocking HTTP (or GraphQL-over-HTTP) request.
pub fn run_http(
    url: &str,
    method: &str,
    headers: &[(String, String)],
    body: &ExecutionBody,
    mode: &ExecutionMode,
    timeout_secs: u64,
    verify_ssl: bool,
    impersonate_browser: bool,
) -> Result<RawResponse, String> {
    let start = Instant::now();
    let (resolved_url, mut resolved_headers, resolved_body) =
        resolve_request(url, headers, body, mode);

    if impersonate_browser {
        resolved_headers = apply_browser_profile(&resolved_headers);
    }

    let req_method = reqwest::Method::from_bytes(method.as_bytes())
        .unwrap_or(reqwest::Method::GET);

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .danger_accept_invalid_certs(!verify_ssl)
        .build()
        .map_err(|e| e.to_string())?;
    let mut req_builder = client.request(req_method, &resolved_url);

    let is_multipart = matches!(resolved_body, ExecutionBody::Multipart(_));
    for (key, value) in &resolved_headers {
        if is_multipart && key.eq_ignore_ascii_case("content-type") {
            continue;
        }
        req_builder = req_builder.header(key.as_str(), value.as_str());
    }

    match &resolved_body {
        ExecutionBody::None => {}
        ExecutionBody::Text(s) => { req_builder = req_builder.body(s.clone()); }
        ExecutionBody::Binary(bytes) => { req_builder = req_builder.body(bytes.clone()); }
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

    Ok(RawResponse { status, status_text, headers: resp_headers, body: body_str, time: elapsed, size })
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
