//! Collection runner — executes all requests in a folder sequentially.
//! Environment changes from each request carry forward to the next.

pub mod data_driven;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use async_channel::Sender;
use http_parser::{Protocol, Request};

use crate::execution::{self, ExecutionBody, ExecutionMode, ExecutionRequest, ExecutionResult};

/// Progress event sent through the channel during a run.
#[derive(Debug, Clone)]
pub enum RunProgress {
    /// A request is about to start.
    Starting { index: usize, total: usize, name: String },
    /// A request completed.
    Completed { index: usize, result: RequestRunResult },
    /// The whole run finished.
    Done,
}

/// Result for a single request in the collection run.
#[derive(Debug, Clone)]
pub struct RequestRunResult {
    pub name: String,
    pub path: PathBuf,
    pub result: Result<ExecutionResult, String>,
}

/// Configuration for a collection run.
pub struct RunConfig {
    pub collection_path: PathBuf,
    pub env_vars: HashMap<String, String>,
    pub stop_on_failure: bool,
}

/// Run all requests in a collection directory sequentially (blocking).
/// Call from a background thread; progress is sent over `tx`.
pub fn run_collection(config: RunConfig, tx: Sender<RunProgress>) {
    let requests = collect_requests(&config.collection_path);
    let total = requests.len();
    let mut env = config.env_vars.clone();

    for (index, (name, path, req)) in requests.into_iter().enumerate() {
        let _ = tx.send_blocking(RunProgress::Starting {
            index,
            total,
            name: name.clone(),
        });

        let exec_req = build_execution_request(&req, &env);
        let result = std::thread::spawn(move || execution::execute(exec_req))
            .join()
            .unwrap_or_else(|_| Err("Request thread panicked".to_string()));

        // Carry forward env changes
        if let Ok(ref res) = result {
            for (k, v) in &res.env_changes {
                env.insert(k.clone(), v.clone());
            }
            for (k, v) in &res.extracted_vars {
                env.insert(k.clone(), v.clone());
            }
        }

        let failed = result.as_ref().map(|r| r.test_results.iter().any(|t| !t.passed)).unwrap_or(true);

        let run_result = RequestRunResult { name, path, result };
        let _ = tx.send_blocking(RunProgress::Completed { index, result: run_result });

        if config.stop_on_failure && failed {
            break;
        }
    }

    let _ = tx.send_blocking(RunProgress::Done);
}

/// Collect all requests from all .http files under `dir`, sorted dirs-first alphabetically.
fn collect_requests(dir: &Path) -> Vec<(String, PathBuf, Request)> {
    let mut out = Vec::new();
    collect_dir(dir, &mut out);
    out
}

fn collect_dir(dir: &Path, out: &mut Vec<(String, PathBuf, Request)>) {
    let mut entries: Vec<std::fs::DirEntry> = match std::fs::read_dir(dir) {
        Ok(it) => it.filter_map(|e| e.ok()).collect(),
        Err(_) => return,
    };
    entries.sort_by_key(|e| {
        let path = e.path();
        (!path.is_dir(), e.file_name().to_string_lossy().to_lowercase())
    });

    for entry in entries {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') { continue; }
        if path.is_dir() {
            collect_dir(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("http") {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(requests) = http_parser::parse(&content) {
                    for (i, req) in requests.into_iter().enumerate() {
                        let req_name = req.meta.name.clone().unwrap_or_else(|| {
                            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("request");
                            if i == 0 { stem.to_string() } else { format!("{} #{}", stem, i + 1) }
                        });
                        out.push((req_name, path.clone(), req));
                    }
                }
            }
        }
    }
}

/// Build an ExecutionRequest from a parsed http_parser::Request + current env vars.
fn build_execution_request(req: &Request, env: &HashMap<String, String>) -> ExecutionRequest {
    let sub = |s: &str| substitute(s, env);

    let url = sub(&req.url);
    let headers: Vec<(String, String)> = req.headers.iter()
        .filter(|h| h.enabled)
        .map(|h| (h.key.clone(), sub(&h.value)))
        .collect();

    let body_str = req.body.as_deref().map(|b| sub(b)).unwrap_or_default();

    let (body, mode) = if req.protocol() == Protocol::GraphQL {
        // Body is JSON like {"query": "...", "variables": {...}}
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body_str) {
            let query = json["query"].as_str().unwrap_or("").to_string();
            let vars = json.get("variables")
                .map(|v| serde_json::to_string(v).unwrap_or_default())
                .unwrap_or_default();
            let op = json["operationName"].as_str().map(|s| s.to_string());
            (ExecutionBody::None, ExecutionMode::GraphQL { query, variables: vars, operation_name: op })
        } else {
            (ExecutionBody::Text(body_str), ExecutionMode::GraphQL {
                query: String::new(), variables: String::new(), operation_name: None,
            })
        }
    } else {
        let eb = if body_str.is_empty() { ExecutionBody::None } else { ExecutionBody::Text(body_str) };
        (eb, ExecutionMode::Http)
    };

    ExecutionRequest {
        method: req.method.as_str().to_string(),
        url,
        headers,
        body,
        mode,
        pre_script: req.scripts.pre_script.clone().unwrap_or_default(),
        post_script: req.scripts.post_script.clone().unwrap_or_default(),
        tests: req.scripts.tests.clone().unwrap_or_default(),
        env_vars: env.clone(),
        variable_extractions: req.meta.variable_extractions.clone(),
        timeout_secs: 30,
        verify_ssl: true,
    }
}

/// Substitute `{{var}}` in `input` using `env`.
pub(crate) fn substitute(input: &str, env: &HashMap<String, String>) -> String {
    let mut result = input.to_string();
    for (key, value) in env {
        result = result.replace(&format!("{{{{{}}}}}", key), value);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_substitute() {
        let mut env = HashMap::new();
        env.insert("base_url".to_string(), "https://api.example.com".to_string());
        env.insert("token".to_string(), "abc123".to_string());
        assert_eq!(substitute("{{base_url}}/users", &env), "https://api.example.com/users");
        assert_eq!(substitute("Bearer {{token}}", &env), "Bearer abc123");
        assert_eq!(substitute("no vars here", &env), "no vars here");
    }

    #[test]
    fn test_collect_requests_empty_dir() {
        let tmp = std::env::temp_dir().join(format!("runner_test_{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().subsec_nanos()));
        std::fs::create_dir_all(&tmp).unwrap();
        let requests = collect_requests(&tmp);
        let _ = std::fs::remove_dir_all(&tmp);
        assert!(requests.is_empty());
    }

    #[test]
    fn test_collect_requests_finds_http_files() {
        use std::io::Write;
        let tmp = std::env::temp_dir().join(format!("runner_test2_{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().subsec_nanos()));
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::File::create(tmp.join("req.http")).unwrap()
            .write_all(b"# @name myReq\nGET https://example.com\n").unwrap();

        let requests = collect_requests(&tmp);
        let _ = std::fs::remove_dir_all(&tmp);
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].0, "myReq");
    }
}
