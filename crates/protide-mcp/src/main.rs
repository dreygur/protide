//! Protide MCP server — JSON-RPC 2.0 over stdio (Model Context Protocol)
//!
//! Usage: pipe JSON-RPC messages to stdin, one per line.
//! Responses are written to stdout, one per line.
//!
//! Exposed tools:
//!   - send_request  Execute an HTTP/GraphQL request and return the full response.

use std::collections::HashMap;
use std::io::{BufRead, Write};

use serde::Deserialize;
use serde_json::{json, Value};

use protide_core::execution::{ExecutionBody, ExecutionMode, ExecutionRequest};

// ── JSON-RPC response ─────────────────────────────────────────────────────────

#[derive(serde::Serialize)]
struct Response {
    jsonrpc: &'static str,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<Value>,
}

impl Response {
    fn ok(id: Value, result: Value) -> Self {
        Self { jsonrpc: "2.0", id, result: Some(result), error: None }
    }
    fn rpc_error(id: Value, code: i32, message: &str) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(json!({"code": code, "message": message})),
        }
    }
    fn tool_error(id: Value, message: impl Into<String>) -> Self {
        Self::ok(
            id,
            json!({"content": [{"type": "text", "text": message.into()}], "isError": true}),
        )
    }
}

// ── Tool input types ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct SendRequestArgs {
    method: String,
    url: String,
    #[serde(default)]
    headers: Vec<HeaderPair>,
    body: Option<String>,
    #[serde(default)]
    env_vars: HashMap<String, String>,
    #[serde(default)]
    pre_script: String,
    #[serde(default)]
    post_script: String,
    #[serde(default)]
    tests: String,
    /// Set to "graphql" to use GraphQL mode; omit for plain HTTP
    #[serde(default)]
    mode: String,
    #[serde(default)]
    graphql_query: String,
    #[serde(default)]
    graphql_variables: String,
    #[serde(default)]
    graphql_operation_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct HeaderPair {
    key: String,
    value: String,
}

// ── Tool schema ───────────────────────────────────────────────────────────────

fn tools_list() -> Value {
    json!({
        "tools": [{
            "name": "send_request",
            "description": "Execute an HTTP or GraphQL request. Returns status, headers, body, timing, and test assertion results.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "method": {
                        "type": "string",
                        "enum": ["GET", "POST", "PUT", "PATCH", "DELETE"],
                        "description": "HTTP method"
                    },
                    "url": {
                        "type": "string",
                        "description": "Request URL. Supports {{variable}} substitution with env_vars."
                    },
                    "headers": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "key": {"type": "string"},
                                "value": {"type": "string"}
                            },
                            "required": ["key", "value"]
                        }
                    },
                    "body": {
                        "type": "string",
                        "description": "Request body string (JSON, XML, plain text, etc.)"
                    },
                    "env_vars": {
                        "type": "object",
                        "additionalProperties": {"type": "string"},
                        "description": "Variables for {{variable}} substitution in url, headers, and body"
                    },
                    "mode": {
                        "type": "string",
                        "enum": ["http", "graphql"],
                        "description": "Request mode. Default: http."
                    },
                    "graphql_query": {
                        "type": "string",
                        "description": "GraphQL query string (required when mode is graphql)"
                    },
                    "graphql_variables": {
                        "type": "string",
                        "description": "GraphQL variables as a JSON string"
                    },
                    "graphql_operation_name": {
                        "type": "string",
                        "description": "GraphQL operation name (optional)"
                    },
                    "pre_script": {
                        "type": "string",
                        "description": "JavaScript pre-request script. Available globals: request.setUrl(url), request.setHeader(name, value), request.removeHeader(name), request.setBody(body), env.set(name, value), env.get(name), env.has(name), console.log(...)"
                    },
                    "post_script": {
                        "type": "string",
                        "description": "JavaScript post-response script. Available globals: response.status, response.statusText, response.body, response.headers, response.time, response.size, response.json(), response.getHeader(name), env.set(name, value), console.log(...)"
                    },
                    "tests": {
                        "type": "string",
                        "description": "JavaScript test assertions. Use expect(value).toBe(x), .toEqual(x), .toBeTruthy(), .toBeFalsy(), .toContain(x), .toHaveLength(n), .toHaveProperty(path, value), .toMatch(pattern), .toBeGreaterThan(n), .toBeLessThan(n), and .not chaining. Example: expect(response.status).toBe(200)"
                    }
                },
                "required": ["method", "url"]
            }
        }]
    })
}

// ── Tool dispatch ─────────────────────────────────────────────────────────────

async fn call_tool(id: Value, params: &Value) -> Response {
    let name = params["name"].as_str().unwrap_or("");
    if name != "send_request" {
        return Response::rpc_error(id, -32602, "Unknown tool");
    }

    let args: SendRequestArgs = match serde_json::from_value(params["arguments"].clone()) {
        Ok(a) => a,
        Err(e) => return Response::tool_error(id, format!("Invalid arguments: {e}")),
    };

    let mode = if args.mode == "graphql" {
        ExecutionMode::GraphQL {
            query: args.graphql_query,
            variables: args.graphql_variables,
            operation_name: args.graphql_operation_name,
        }
    } else {
        ExecutionMode::Http
    };

    let req = ExecutionRequest {
        method: args.method,
        url: args.url,
        headers: args.headers.into_iter().map(|h| (h.key, h.value)).collect(),
        body: args.body.map(ExecutionBody::Text).unwrap_or(ExecutionBody::None),
        mode,
        pre_script: args.pre_script,
        post_script: args.post_script,
        tests: args.tests,
        env_vars: args.env_vars,
        variable_extractions: vec![],
        timeout_secs: 30,
        verify_ssl: true,
    };

    let result = tokio::task::spawn_blocking(move || protide_core::execution::execute(req))
        .await
        .unwrap_or_else(|_| Err("Execution thread panicked".into()));

    match result {
        Ok(r) => {
            let body = serde_json::to_string_pretty(&r).unwrap_or_else(|e| e.to_string());
            Response::ok(id, json!({"content": [{"type": "text", "text": body}]}))
        }
        Err(e) => Response::tool_error(id, e),
    }
}

// ── Main loop ─────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) if !l.trim().is_empty() => l,
            _ => continue,
        };

        let msg: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        // Notifications (no id field) require no response
        let id = match msg.get("id").cloned() {
            Some(id) => id,
            None => continue,
        };

        let method = msg["method"].as_str().unwrap_or("");
        let params = msg.get("params").cloned().unwrap_or(Value::Null);

        let resp = match method {
            "initialize" => Response::ok(
                id,
                json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {"tools": {}},
                    "serverInfo": {
                        "name": "protide",
                        "version": env!("CARGO_PKG_VERSION")
                    }
                }),
            ),
            "tools/list" => Response::ok(id, tools_list()),
            "tools/call" => call_tool(id, &params).await,
            _ => Response::rpc_error(id, -32601, "Method not found"),
        };

        let _ = writeln!(stdout, "{}", serde_json::to_string(&resp).unwrap());
        let _ = stdout.flush();
    }
}
