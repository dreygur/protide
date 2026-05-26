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
        .map_err(|e| {
            // char-boundary-safe preview — byte slice would panic on multi-byte UTF-8
            let preview_end = body.char_indices().nth(300).map(|(i, _)| i).unwrap_or(body.len());
            format!("Invalid JSON: {} — body: {}", e, &body[..preview_end])
        })?;

    // Unwrap batch array (server may respond as [{...}] even for non-batch requests)
    let resp = if response_json.is_array() {
        response_json.as_array()
            .and_then(|a| a.first())
            .cloned()
            .unwrap_or_default()
    } else {
        response_json
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

/// A procedure discovered by tRPC schema introspection.
#[derive(Debug, Clone)]
pub struct TrpcSchemaProc {
    pub name: String,
    pub is_mutation: bool,
}

/// Fetch raw schema JSON from a tRPC base URL.
/// Tries the base URL, then `{base_url}/schema`.
pub fn fetch_trpc_schema_raw(base_url: &str) -> Result<String, String> {
    let base = base_url.trim_end_matches('/');
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("HTTP client: {}", e))?;

    for url in &[base.to_string(), format!("{}/schema", base)] {
        let resp = client.get(url.as_str())
            .header("Accept", "application/json")
            .send();
        if let Ok(r) = resp {
            if r.status().is_success() {
                return r.text().map_err(|e| format!("Read error: {}", e));
            }
        }
    }
    Err("No schema endpoint responded. Paste schema JSON via the add row, or add procedures manually.".to_string())
}

/// Parse a tRPC schema JSON payload into a flat list of procedures.
///
/// Recognises four formats (tried in order):
///
/// 1. tRPC Panel — `{"procedures": {"users.login": {"procedureType": "mutation"}}}`
/// 2. Nested router — `{"users": {"login": {"type": "mutation"}}}`
/// 3. Flat string values — `{"users.login": "mutation"}`
/// 4. Array — `[{"path": "users.login", "type": "mutation"}]`
pub fn parse_trpc_schema(json: &str) -> Result<Vec<TrpcSchemaProc>, String> {
    let val: serde_json::Value = serde_json::from_str(json)
        .map_err(|e| format!("Invalid JSON: {}", e))?;

    // Format 1: {"procedures": {"path": {"procedureType": "query|mutation"}}}
    if let Some(procs_obj) = val.get("procedures").and_then(|v| v.as_object()) {
        let procs: Vec<TrpcSchemaProc> = procs_obj.iter().map(|(key, def)| {
            let kind = def.get("procedureType")
                .or_else(|| def.get("type"))
                .and_then(|t| t.as_str())
                .unwrap_or("query");
            TrpcSchemaProc { name: key.clone(), is_mutation: kind == "mutation" }
        }).collect();
        if !procs.is_empty() { return Ok(procs); }
    }

    // Format 2: nested router object {"router": {"proc": {"type": "query|mutation"}}}
    if let Some(obj) = val.as_object() {
        let mut procs = Vec::new();
        for (router, router_val) in obj {
            if router.starts_with('_') { continue; }
            if let Some(router_obj) = router_val.as_object() {
                // Only treat as nested router if values are objects (not strings/nulls)
                let has_object_vals = router_obj.values()
                    .any(|v| v.is_object() && v.get("procedureType").is_none());
                if !has_object_vals { continue; }
                for (proc_name, proc_val) in router_obj {
                    if proc_name.starts_with('_') { continue; }
                    let kind = proc_val.get("type")
                        .or_else(|| proc_val.get("procedureType"))
                        .and_then(|t| t.as_str())
                        .unwrap_or("query");
                    procs.push(TrpcSchemaProc {
                        name: format!("{}.{}", router, proc_name),
                        is_mutation: kind == "mutation",
                    });
                }
            }
        }
        if !procs.is_empty() { return Ok(procs); }
    }

    // Format 3: flat string map {"users.login": "mutation"}
    if let Some(obj) = val.as_object() {
        let procs: Vec<TrpcSchemaProc> = obj.iter().filter_map(|(key, v)| {
            v.as_str().map(|kind| TrpcSchemaProc {
                name: key.clone(),
                is_mutation: kind == "mutation",
            })
        }).collect();
        if !procs.is_empty() { return Ok(procs); }
    }

    // Format 4: array [{"path": "users.login", "type": "mutation"}]
    if let Some(arr) = val.as_array() {
        let procs: Vec<TrpcSchemaProc> = arr.iter().filter_map(|item| {
            let name = item.get("path")
                .or_else(|| item.get("name"))
                .and_then(|v| v.as_str())?;
            let kind = item.get("type")
                .or_else(|| item.get("procedureType"))
                .and_then(|v| v.as_str())
                .unwrap_or("query");
            Some(TrpcSchemaProc { name: name.to_string(), is_mutation: kind == "mutation" })
        }).collect();
        if !procs.is_empty() { return Ok(procs); }
    }

    Err("No recognised procedure format in schema JSON. Expected 'procedures' key, nested router object, flat string map, or array.".to_string())
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

    #[test]
    fn test_parse_schema_format1_procedures_key() {
        let json = r#"{"procedures":{"users.login":{"procedureType":"mutation"},"users.getAll":{"procedureType":"query"}}}"#;
        let procs = parse_trpc_schema(json).unwrap();
        assert_eq!(procs.len(), 2);
        let login = procs.iter().find(|p| p.name == "users.login").unwrap();
        assert!(login.is_mutation);
        let get_all = procs.iter().find(|p| p.name == "users.getAll").unwrap();
        assert!(!get_all.is_mutation);
    }

    #[test]
    fn test_parse_schema_format3_flat_strings() {
        let json = r#"{"users.login":"mutation","users.getAll":"query","posts.create":"mutation"}"#;
        let procs = parse_trpc_schema(json).unwrap();
        assert_eq!(procs.len(), 3);
        assert!(procs.iter().any(|p| p.name == "users.login" && p.is_mutation));
        assert!(procs.iter().any(|p| p.name == "posts.create" && p.is_mutation));
        assert!(procs.iter().any(|p| p.name == "users.getAll" && !p.is_mutation));
    }

    #[test]
    fn test_parse_schema_format4_array() {
        let json = r#"[{"path":"users.login","type":"mutation"},{"path":"billing.getInvoices","type":"query"}]"#;
        let procs = parse_trpc_schema(json).unwrap();
        assert_eq!(procs.len(), 2);
        assert!(procs.iter().any(|p| p.name == "billing.getInvoices" && !p.is_mutation));
    }

    #[test]
    fn test_parse_schema_invalid_json() {
        assert!(parse_trpc_schema("not json").is_err());
    }

    #[test]
    fn test_parse_schema_unrecognised_format() {
        assert!(parse_trpc_schema(r#"{"foo": 42}"#).is_err());
    }
}
