mod http;
pub mod ws;

use std::collections::HashMap;
use std::time::Duration;

use http_parser::VariableExtraction;

use crate::chaining;
use crate::scripting::context::{RequestData, ResponseData as ScriptResponseData};
use crate::scripting::results::TestResult;
use crate::scripting::ScriptEngine;

pub use http::run_http;

/// Body of an HTTP request
#[derive(Debug, Clone)]
pub enum ExecutionBody {
    None,
    Text(String),
    Multipart(Vec<FormPart>),
    Binary(Vec<u8>),
}

impl ExecutionBody {
    pub fn as_text(&self) -> Option<String> {
        match self {
            ExecutionBody::Text(s) => Some(s.clone()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FormPart {
    pub name: String,
    pub value: FormPartValue,
}

#[derive(Debug, Clone)]
pub enum FormPartValue {
    Text(String),
    File(std::path::PathBuf),
}

/// Protocol mode for the request
#[derive(Debug, Clone)]
pub enum ExecutionMode {
    Http,
    GraphQL {
        query: String,
        variables: String,
        operation_name: Option<String>,
    },
}

/// Everything needed to execute a request — all values already env-substituted by the UI
pub struct ExecutionRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: ExecutionBody,
    pub mode: ExecutionMode,
    pub pre_script: String,
    pub post_script: String,
    pub tests: String,
    /// Active environment variables for script context
    pub env_vars: HashMap<String, String>,
    pub variable_extractions: Vec<VariableExtraction>,
}

/// Full result of executing a request
#[derive(serde::Serialize)]
pub struct ExecutionResult {
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
    /// Elapsed time in milliseconds
    #[serde(serialize_with = "ser_duration_millis")]
    pub time: Duration,
    pub size: usize,
    pub test_results: Vec<TestResult>,
    pub console_output: Vec<String>,
    pub env_changes: Vec<(String, String)>,
    pub extracted_vars: Vec<(String, String)>,
}

fn ser_duration_millis<S: serde::Serializer>(d: &Duration, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_u64(d.as_millis() as u64)
}

/// Execute an HTTP/GraphQL request including pre/post scripts and variable extraction.
/// Blocking — must be called from a background thread (e.g., std::thread::spawn).
pub fn execute(req: ExecutionRequest) -> Result<ExecutionResult, String> {
    let mut url = req.url.clone();
    let mut headers = req.headers.clone();
    let mut body = req.body.clone();
    let mut console_output: Vec<String> = Vec::new();
    let mut env_changes: Vec<(String, String)> = Vec::new();

    // 1. Pre-script: may modify url / headers / body
    if !req.pre_script.trim().is_empty() {
        let engine = ScriptEngine::new()
            .map_err(|e| format!("Script engine error: {}", e))?;

        let script_req = RequestData::new(&req.method, &url)
            .with_headers(headers.clone())
            .with_body(body.as_text().unwrap_or_default());
        let mut ctx = crate::scripting::ScriptContext::new()
            .with_request(script_req)
            .with_env(req.env_vars.clone());

        let outcome = engine
            .run_pre_script(&req.pre_script, &mut ctx)
            .map_err(|e| format!("Pre-script error: {}", e))?;

        if !outcome.success {
            if let Some(err) = outcome.error {
                return Err(format!("Pre-script error: {}", err.message));
            }
        }

        console_output.extend(outcome.console_output);
        env_changes.extend(outcome.env_changes);

        if let Some(new_url) = outcome.modified_request.url {
            url = new_url;
        }
        for (name, value) in outcome.modified_request.headers_to_set {
            headers.retain(|(k, _)| !k.eq_ignore_ascii_case(&name));
            headers.push((name, value));
        }
        for name in &outcome.modified_request.headers_to_remove {
            headers.retain(|(k, _)| !k.eq_ignore_ascii_case(name));
        }
        if let Some(new_body) = outcome.modified_request.body {
            body = ExecutionBody::Text(new_body);
        }
    }

    // 2. Execute HTTP
    let raw = run_http(&url, &req.method, &headers, &body, &req.mode)?;

    // 3. Post-script + tests
    let mut test_results: Vec<TestResult> = Vec::new();
    if !req.post_script.trim().is_empty() || !req.tests.trim().is_empty() {
        if let Ok(engine) = ScriptEngine::new() {
            let script_resp =
                ScriptResponseData::new(raw.status, &raw.status_text, raw.body.clone())
                    .with_headers(raw.headers.clone())
                    .with_time(raw.time.as_millis() as u64)
                    .with_size(raw.size);

            let mut ctx = crate::scripting::ScriptContext::new().with_env(req.env_vars.clone());
            ctx.set_response(script_resp);

            if !req.post_script.trim().is_empty() {
                if let Ok(outcome) = engine.run_post_script(&req.post_script, &mut ctx) {
                    console_output.extend(outcome.console_output);
                    env_changes.extend(outcome.env_changes);
                }
            }
            if !req.tests.trim().is_empty() {
                if let Ok(outcome) = engine.run_tests(&req.tests, &mut ctx) {
                    console_output.extend(outcome.console_output);
                    test_results = outcome.test_results;
                }
            }
        }
    }

    // 4. Variable extraction via @set JSONPath annotations
    let extracted_vars: Vec<(String, String)> = if !req.variable_extractions.is_empty() {
        chaining::extract_variables(&raw.body, &req.variable_extractions)
            .into_iter()
            .filter(|r| r.success)
            .map(|r| (r.name, r.value))
            .collect()
    } else {
        Vec::new()
    };

    Ok(ExecutionResult {
        status: raw.status,
        status_text: raw.status_text,
        headers: raw.headers,
        body: raw.body,
        time: raw.time,
        size: raw.size,
        test_results,
        console_output,
        env_changes,
        extracted_vars,
    })
}
