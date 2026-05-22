//! tRPC protocol support — tRPC v10/v11 HTTP adapter
//!
//! Procedure field format: "mutation.proc.name" or "query.proc.name"
//! The prefix determines the HTTP method; the suffix is appended to the base URL.

use std::time::Duration;

/// Execute a tRPC call.
///
/// `procedure`: "mutation.users.login" → POST base_url/users.login
///              "query.users.get"      → GET  base_url/users.get?input={"json":params}
///              "proc.name"            → defaults to query (GET)
pub fn execute_trpc(
    base_url: &str,
    procedure: &str,
    params: &str,
    headers: Vec<(String, String)>,
) -> Result<(String, Duration, u16), String> {
    let start = std::time::Instant::now();

    let (is_mutation, proc_name) = if let Some(name) = procedure.strip_prefix("mutation.") {
        (true, name)
    } else if let Some(name) = procedure.strip_prefix("query.") {
        (false, name)
    } else {
        (false, procedure)
    };

    let params_value: serde_json::Value = serde_json::from_str(params)
        .unwrap_or(serde_json::json!(null));

    let url = format!("{}/{}", base_url.trim_end_matches('/'), proc_name);

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let response = if is_mutation {
        // tRPC v10: POST {"json": params}
        let body = serde_json::json!({"json": params_value});
        let mut req = client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body);
        for (k, v) in &headers {
            req = req.header(k.as_str(), v.as_str());
        }
        req.send().map_err(|e| format!("Request failed: {}", e))?
    } else {
        // tRPC v10: GET ?input={"json":params}
        let input = serde_json::json!({"json": params_value});
        let input_str = serde_json::to_string(&input)
            .map_err(|e| format!("Serialize error: {}", e))?;
        let mut req = client
            .get(&url)
            .query(&[("input", &input_str)]);
        for (k, v) in &headers {
            req = req.header(k.as_str(), v.as_str());
        }
        req.send().map_err(|e| format!("Request failed: {}", e))?
    };

    let elapsed = start.elapsed();
    let status = response.status().as_u16();
    let body = response.text()
        .map_err(|e| format!("Failed to read response: {}", e))?;

    let response_json: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("Invalid JSON: {} — body: {}", e, &body[..body.len().min(300)]))?;

    // Unwrap batch array (server may respond as [{...}] even for non-batch requests)
    let resp = if let Some(arr) = response_json.as_array() {
        arr.first().cloned().unwrap_or_default()
    } else {
        response_json.clone()
    };

    // Error: tRPC v10 wraps under error.json; fall back to error itself for v9/plain
    if let Some(error) = resp.get("error") {
        let inner = error.get("json").unwrap_or(error);
        let code = inner.get("code").and_then(|c| c.as_i64()).unwrap_or(-1);
        let message = inner.get("message").and_then(|m| m.as_str()).unwrap_or("Unknown error");
        let data = inner.get("data");
        return Err(if let Some(data) = data {
            format!("tRPC error ({}): {} — data: {}", code, message, data)
        } else {
            format!("tRPC error ({}): {}", code, message)
        });
    }

    // Success: tRPC v10 result is at result.data.json; fall back to result.data or whole resp
    let result = resp.get("result")
        .and_then(|r| r.get("data"))
        .and_then(|d| d.get("json"))
        .cloned()
        .or_else(|| resp.get("result").and_then(|r| r.get("data")).cloned())
        .or_else(|| resp.get("result").cloned())
        .unwrap_or(resp);

    let formatted = serde_json::to_string_pretty(&result).unwrap_or(body);
    Ok((formatted, elapsed, status))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_procedure_parsing_mutation() {
        // Verify mutation prefix strips correctly
        let procedure = "mutation.users.login";
        let (is_mutation, name) = if let Some(n) = procedure.strip_prefix("mutation.") {
            (true, n)
        } else {
            (false, procedure)
        };
        assert!(is_mutation);
        assert_eq!(name, "users.login");
    }

    #[test]
    fn test_procedure_parsing_query() {
        let procedure = "query.users.getProfile";
        let (is_mutation, name) = if let Some(n) = procedure.strip_prefix("mutation.") {
            (true, n)
        } else if let Some(n) = procedure.strip_prefix("query.") {
            (false, n)
        } else {
            (false, procedure)
        };
        assert!(!is_mutation);
        assert_eq!(name, "users.getProfile");
    }

    #[test]
    fn test_error_extraction_v10() {
        let resp: serde_json::Value = serde_json::json!({
            "error": {
                "json": {
                    "message": "UNAUTHORIZED",
                    "code": -32600,
                    "data": { "code": "UNAUTHORIZED", "httpStatus": 401 }
                }
            }
        });
        let error = resp.get("error").unwrap();
        let inner = error.get("json").unwrap_or(error);
        let code = inner.get("code").and_then(|c| c.as_i64()).unwrap_or(-1);
        let message = inner.get("message").and_then(|m| m.as_str()).unwrap_or("Unknown error");
        assert_eq!(code, -32600);
        assert_eq!(message, "UNAUTHORIZED");
    }

    #[test]
    fn test_batch_response_unwrap() {
        let resp: serde_json::Value = serde_json::json!([{
            "result": { "data": { "json": { "token": "abc123" } } }
        }]);
        let inner = resp.as_array().unwrap().first().cloned().unwrap();
        let result = inner.get("result")
            .and_then(|r| r.get("data"))
            .and_then(|d| d.get("json"))
            .cloned()
            .unwrap();
        assert_eq!(result["token"], "abc123");
    }
}
