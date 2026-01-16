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
    let client = reqwest::blocking::Client::new();
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
    if let Some(resp_id) = response_json.get("id").and_then(|id| id.as_str()) {
        if resp_id != request_id {
            return Err(format!("Response ID mismatch: expected {}, got {}", request_id, resp_id));
        }
    }

    // Pretty-print the response
    let formatted = serde_json::to_string_pretty(&response_json)
        .unwrap_or(body);

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
