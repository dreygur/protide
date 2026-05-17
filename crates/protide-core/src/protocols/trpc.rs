//! tRPC protocol support using JSON-RPC 2.0 over HTTP

use std::time::Duration;

/// Build a JSON-RPC 2.0 request for tRPC
///
/// # Arguments
/// * `procedure` - The tRPC procedure name (e.g., "query.getUser", "mutation.createPost")
/// * `params` - The parameters as a JSON value
/// * `id` - Request ID (typically a UUID string)
///
/// # Returns
/// JSON-RPC 2.0 request object
pub fn build_trpc_request(
    procedure: &str,
    params: serde_json::Value,
    id: String,
) -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": procedure,
        "params": params,
    })
}

/// Execute a tRPC call via HTTP POST
///
/// # Arguments
/// * `url` - The tRPC endpoint URL (typically ends with /trpc)
/// * `procedure` - The procedure name (e.g., "query.getUser")
/// * `params` - JSON string of parameters
/// * `headers` - Additional HTTP headers
///
/// # Returns
/// Result containing (response_body, elapsed_time, status_code) or error string
pub fn execute_trpc(
    url: &str,
    procedure: &str,
    params: &str,
    headers: Vec<(String, String)>,
) -> Result<(String, Duration, u16), String> {
    let start = std::time::Instant::now();

    // Parse params JSON
    let params_value: serde_json::Value = serde_json::from_str(params)
        .unwrap_or(serde_json::json!({}));

    // Build JSON-RPC 2.0 request
    let request_id = uuid::Uuid::new_v4().to_string();
    let request_body = build_trpc_request(procedure, params_value, request_id.clone());

    // Create blocking HTTP client
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;
    let mut req = client
        .post(url)
        .header("Content-Type", "application/json")
        .json(&request_body);

    // Add custom headers
    for (key, value) in headers {
        req = req.header(key, value);
    }

    // Send request
    let response = req.send()
        .map_err(|e| format!("Request failed: {}", e))?;

    let elapsed = start.elapsed();
    let status = response.status().as_u16();

    // Read response body
    let body = response.text()
        .map_err(|e| format!("Failed to read response: {}", e))?;

    // Parse and validate JSON-RPC response
    let response_json: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("Invalid JSON response: {}", e))?;

    // Check for JSON-RPC error
    if let Some(error) = response_json.get("error") {
        let code = error.get("code").and_then(|c| c.as_i64()).unwrap_or(-1);
        let message = error.get("message").and_then(|m| m.as_str()).unwrap_or("Unknown error");
        let data = error.get("data");

        let error_msg = if let Some(data) = data {
            format!("tRPC error ({}): {} - {:?}", code, message, data)
        } else {
            format!("tRPC error ({}): {}", code, message)
        };

        return Err(error_msg);
    }

    // Verify response ID matches request ID
    if let Some(resp_id) = response_json.get("id").and_then(|id| id.as_str())
        && resp_id != request_id {
            return Err(format!("Response ID mismatch: expected {}, got {}", request_id, resp_id));
        }

    // Pretty-print the response
    let formatted = serde_json::to_string_pretty(&response_json)
        .unwrap_or(body);

    Ok((formatted, elapsed, status))
}

/// A single call in a tRPC batch request
pub struct BatchCall {
    pub procedure: String,
    pub params: String,
}

/// Execute multiple tRPC calls in a single batch HTTP request (tRPC v11 native batch format).
///
/// Sends `POST {base}/{proc0},{proc1}?batch=1` with body `{"0":{"json":p0},"1":{"json":p1},...}`.
///
/// # Returns
/// Result containing (response_body, elapsed_time, status_code) or error string
pub fn execute_trpc_batch(
    base_url: &str,
    calls: &[BatchCall],
    headers: Vec<(String, String)>,
) -> Result<(String, Duration, u16), String> {
    if calls.is_empty() {
        return Err("No procedures in batch".to_string());
    }

    let start = std::time::Instant::now();

    let procedures: Vec<&str> = calls.iter().map(|c| c.procedure.as_str()).collect();
    let url = format!("{}/{}?batch=1", base_url, procedures.join(","));

    let mut body = serde_json::Map::new();
    for (i, call) in calls.iter().enumerate() {
        let params: serde_json::Value = serde_json::from_str(&call.params)
            .unwrap_or(serde_json::Value::Null);
        body.insert(i.to_string(), serde_json::json!({"json": params}));
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let mut req = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&serde_json::Value::Object(body));

    for (key, value) in headers {
        req = req.header(key, value);
    }

    let response = req.send()
        .map_err(|e| format!("Request failed: {}", e))?;

    let elapsed = start.elapsed();
    let status = response.status().as_u16();
    let body_text = response.text()
        .map_err(|e| format!("Failed to read response: {}", e))?;

    let response_json: serde_json::Value = serde_json::from_str(&body_text)
        .map_err(|e| format!("Invalid JSON response: {}", e))?;

    // Report the first error found in the batch response array
    if let Some(arr) = response_json.as_array() {
        for (i, item) in arr.iter().enumerate() {
            if let Some(error) = item.get("error") {
                let code = error.get("code").and_then(|c| c.as_i64()).unwrap_or(-1);
                let message = error.get("message").and_then(|m| m.as_str()).unwrap_or("Unknown error");
                return Err(format!("tRPC batch error in call {} (code {}): {}", i, code, message));
            }
        }
    }

    let formatted = serde_json::to_string_pretty(&response_json).unwrap_or(body_text);
    Ok((formatted, elapsed, status))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_trpc_request() {
        let request = build_trpc_request(
            "query.getUser",
            serde_json::json!({"userId": 123}),
            "test-id-123".to_string(),
        );

        assert_eq!(request["jsonrpc"], "2.0");
        assert_eq!(request["id"], "test-id-123");
        assert_eq!(request["method"], "query.getUser");
        assert_eq!(request["params"]["userId"], 123);
    }

    #[test]
    fn test_build_trpc_request_with_empty_params() {
        let request = build_trpc_request(
            "mutation.logout",
            serde_json::json!({}),
            "test-id-456".to_string(),
        );

        assert_eq!(request["jsonrpc"], "2.0");
        assert_eq!(request["method"], "mutation.logout");
        assert_eq!(request["params"], serde_json::json!({}));
    }
}
