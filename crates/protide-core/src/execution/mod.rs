pub(crate) mod http;
pub mod sio;
mod sio_codec;
#[cfg(test)]
pub(crate) mod test_server;
pub mod ws;

use std::collections::HashMap;
use std::time::Duration;

use http_parser::VariableExtraction;

use crate::chaining;
use crate::scripting::ScriptEngine;
use crate::scripting::context::{RequestData, ResponseData as ScriptResponseData};
use crate::scripting::results::TestResult;

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

/// Everything needed to execute a request - all values already env-substituted by the UI
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
    pub timeout_secs: u64,
    pub verify_ssl: bool,
    /// Send request with Chrome-profile TLS fingerprint (JA3/JA4) and HTTP/2 SETTINGS.
    pub impersonate_browser: bool,
}

/// Full result of executing a request
#[derive(Debug, Clone, serde::Serialize)]
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
    /// Error messages for `@set` extractions that failed (e.g. JSONPath
    /// didn't match anything). Kept separate from `extracted_vars` so
    /// callers can surface failures instead of them vanishing silently.
    pub extraction_errors: Vec<String>,
}

fn ser_duration_millis<S: serde::Serializer>(d: &Duration, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_u64(d.as_millis() as u64)
}

/// Console note for a post-response script that ran but reported failure.
///
/// A script that fails *after* the response is in hand must not fail the
/// request - the user still wants the response - but the error has to reach
/// the console instead of vanishing, or a typo'd script looks like a script
/// that simply did nothing.
fn failure_note(kind: &str, outcome: &crate::scripting::ScriptOutcome) -> Option<String> {
    match &outcome.error {
        Some(err) if !outcome.success => Some(format!("{} error: {}", kind, err.message)),
        _ => None,
    }
}

/// Execute an HTTP/GraphQL request including pre/post scripts and variable extraction.
/// Blocking - must be called from a background thread (e.g., std::thread::spawn).
pub fn execute(req: ExecutionRequest) -> Result<ExecutionResult, String> {
    let mut url = req.url.clone();
    let mut headers = req.headers.clone();
    let mut body = req.body.clone();
    let mut console_output: Vec<String> = Vec::new();
    let mut env_changes: Vec<(String, String)> = Vec::new();

    // 1. Pre-script: may modify url / headers / body
    if !req.pre_script.trim().is_empty() {
        let engine = ScriptEngine::new().map_err(|e| format!("Script engine error: {}", e))?;

        let script_req = RequestData::new(&req.method, &url)
            .with_headers(headers.clone())
            .with_body(body.as_text().unwrap_or_default());
        let mut ctx = crate::scripting::ScriptContext::new()
            .with_request(script_req)
            .with_env(req.env_vars.clone());

        let outcome = engine
            .run_pre_script(&req.pre_script, &mut ctx)
            .map_err(|e| format!("Pre-script error: {}", e))?;

        if !outcome.success
            && let Some(err) = outcome.error
        {
            return Err(format!("Pre-script error: {}", err.message));
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
    let raw = run_http(
        &url,
        &req.method,
        &headers,
        &body,
        &req.mode,
        req.timeout_secs,
        req.verify_ssl,
        req.impersonate_browser,
    )?;

    // 3. Post-script + tests
    let mut test_results: Vec<TestResult> = Vec::new();
    if !req.post_script.trim().is_empty() || !req.tests.trim().is_empty() {
        match ScriptEngine::new() {
            Err(e) => console_output.push(format!("Script engine error: {}", e)),
            Ok(engine) => {
                let script_resp =
                    ScriptResponseData::new(raw.status, &raw.status_text, raw.body.clone())
                        .with_headers(raw.headers.clone())
                        .with_time(raw.time.as_millis() as u64)
                        .with_size(raw.size);

                let mut ctx = crate::scripting::ScriptContext::new().with_env(req.env_vars.clone());
                ctx.set_response(script_resp);

                if !req.post_script.trim().is_empty() {
                    match engine.run_post_script(&req.post_script, &mut ctx) {
                        Ok(outcome) => {
                            let note = failure_note("Post-script", &outcome);
                            console_output.extend(outcome.console_output);
                            env_changes.extend(outcome.env_changes);
                            if let Some(note) = note {
                                console_output.push(note);
                            }
                        }
                        Err(e) => console_output.push(format!("Post-script error: {}", e)),
                    }
                }
                if !req.tests.trim().is_empty() {
                    match engine.run_tests(&req.tests, &mut ctx) {
                        Ok(outcome) => {
                            let note = failure_note("Tests", &outcome);
                            console_output.extend(outcome.console_output);
                            if let Some(note) = note {
                                console_output.push(note);
                            }
                            test_results = outcome.test_results;
                        }
                        Err(e) => console_output.push(format!("Tests error: {}", e)),
                    }
                }
            }
        }
    }

    // 4. Variable extraction via @set JSONPath annotations
    let mut extracted_vars: Vec<(String, String)> = Vec::new();
    let mut extraction_errors: Vec<String> = Vec::new();
    if !req.variable_extractions.is_empty() {
        for r in chaining::extract_variables(&raw.body, &req.variable_extractions) {
            if r.success {
                extracted_vars.push((r.name, r.value));
            } else {
                extraction_errors.push(format!(
                    "@set {}: {}",
                    r.name,
                    r.error.unwrap_or_else(|| "extraction failed".to_string())
                ));
            }
        }
    }

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
        extraction_errors,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::test_server::TestServer;

    /// A plain GET against `url` with everything optional left empty.
    fn request(url: String) -> ExecutionRequest {
        ExecutionRequest {
            method: "GET".to_string(),
            url,
            headers: Vec::new(),
            body: ExecutionBody::None,
            mode: ExecutionMode::Http,
            pre_script: String::new(),
            post_script: String::new(),
            tests: String::new(),
            env_vars: HashMap::new(),
            variable_extractions: Vec::new(),
            timeout_secs: 5,
            verify_ssl: true,
            impersonate_browser: false,
        }
    }

    /// A failed `@set` extraction must be reported via `extraction_errors`
    /// instead of silently vanishing from `extracted_vars`.
    #[test]
    fn test_failed_extraction_surfaces_in_extraction_errors() {
        let server = TestServer::json(r#"{"items": [1, 2, 3]}"#);

        let req = ExecutionRequest {
            variable_extractions: vec![
                VariableExtraction {
                    name: "good".to_string(),
                    expression: "$.items[0]".to_string(),
                },
                VariableExtraction {
                    name: "bad".to_string(),
                    expression: "$.items[10]".to_string(),
                },
            ],
            ..request(server.url("/"))
        };

        let result = execute(req).expect("execute should succeed");

        assert_eq!(
            result.extracted_vars,
            vec![("good".to_string(), "1".to_string())]
        );
        assert_eq!(result.extraction_errors.len(), 1);
        assert!(result.extraction_errors[0].contains("bad"));
    }

    /// Everything a pre-script changes must reach the wire: URL, headers
    /// (set and removed, case-insensitively) and body.
    #[test]
    fn pre_script_edits_reach_the_wire() {
        let server = TestServer::json("{}");
        let req = ExecutionRequest {
            method: "POST".to_string(),
            headers: vec![
                ("X-Drop-Me".to_string(), "old".to_string()),
                ("X-Keep".to_string(), "kept".to_string()),
            ],
            body: ExecutionBody::Text("original".to_string()),
            pre_script: format!(
                "request.setUrl('{}');\
                 request.setHeader('X-Added', 'added');\
                 request.removeHeader('x-drop-me');\
                 request.setBody('rewritten');\
                 console.log('pre ran');",
                server.url("/rewritten")
            ),
            ..request(server.url("/original"))
        };

        let result = execute(req).expect("execute");
        assert!(result.console_output.iter().any(|l| l.contains("pre ran")));

        let sent = server.only_request();
        assert_eq!(sent.target, "/rewritten");
        assert_eq!(sent.body, "rewritten");
        assert_eq!(sent.header("X-Added"), Some("added"));
        assert_eq!(sent.header("X-Keep"), Some("kept"));
        assert_eq!(sent.header("X-Drop-Me"), None);
    }

    /// A pre-script that throws must abort before anything is sent - the
    /// request was never fully prepared.
    #[test]
    fn failing_pre_script_aborts_before_sending() {
        let server = TestServer::json("{}");
        let req = ExecutionRequest {
            pre_script: "throw new Error('nope');".to_string(),
            ..request(server.url("/"))
        };
        let err = execute(req).expect_err("must not send");
        assert!(err.contains("Pre-script"), "unhelpful error: {err}");
        assert!(server.requests().is_empty(), "request was sent anyway");
    }

    #[test]
    fn post_script_env_changes_and_tests_are_reported() {
        let server = TestServer::json(r#"{"id":7}"#);
        let req = ExecutionRequest {
            post_script: "env.set('id', String(response.json().id));".to_string(),
            tests: "expect(response.status).toBe(200);\nexpect(response.json().id).toBe(7);"
                .to_string(),
            ..request(server.url("/"))
        };

        let result = execute(req).expect("execute");
        assert_eq!(
            result.env_changes,
            vec![("id".to_string(), "7".to_string())]
        );
        assert_eq!(result.test_results.len(), 2);
        assert!(result.test_results.iter().all(|t| t.passed));
    }

    /// A post-response script that blows up must not fail the request (the
    /// response is already in hand) but the error must still be reported -
    /// otherwise a typo'd script is indistinguishable from one that did
    /// nothing at all.
    #[test]
    fn broken_post_script_is_reported_without_failing_the_request() {
        let server = TestServer::json("{}");
        let req = ExecutionRequest {
            post_script: "this is ( not valid javascript".to_string(),
            ..request(server.url("/"))
        };

        let result = execute(req).expect("response must still be delivered");
        assert_eq!(result.status, 200);
        assert!(
            result
                .console_output
                .iter()
                .any(|l| l.contains("Post-script error")),
            "no diagnostic in console: {:?}",
            result.console_output
        );
    }

    /// Same for the tests block: a broken `@tests` script must not silently
    /// report "no tests ran".
    #[test]
    fn broken_tests_script_is_reported() {
        let server = TestServer::json("{}");
        let req = ExecutionRequest {
            tests: "expect(response.nope.deep).toBe(1);".to_string(),
            ..request(server.url("/"))
        };

        let result = execute(req).expect("response must still be delivered");
        assert!(result.test_results.is_empty());
        assert!(
            result
                .console_output
                .iter()
                .any(|l| l.contains("Tests error")),
            "no diagnostic in console: {:?}",
            result.console_output
        );
    }

    /// A transport failure must surface as an error rather than a fabricated
    /// response.
    #[test]
    fn transport_failure_is_an_error() {
        let server = TestServer::spawn(vec![String::new()]);
        execute(request(server.url("/"))).expect_err("closed connection must error");
    }
}
