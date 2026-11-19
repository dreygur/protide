//! Collection runner - executes all requests in a folder sequentially.
//! Environment changes from each request carry forward to the next.

pub mod data_driven;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use async_channel::Sender;
use http_parser::{Protocol, Request};

use crate::execution::{self, ExecutionBody, ExecutionMode, ExecutionRequest, ExecutionResult};
use crate::scripting::trust::TrustStore;

/// Progress event sent through the channel during a run.
#[derive(Debug, Clone)]
pub enum RunProgress {
    /// A request is about to start.
    Starting {
        index: usize,
        total: usize,
        name: String,
    },
    /// A request completed.
    Completed {
        index: usize,
        result: RequestRunResult,
    },
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
    /// Scripts the user agreed to run; any other script refuses its request.
    pub trust: TrustStore,
}

/// Refuse a request read from `path` whose scripts the user has not
/// trusted. A run has no one to ask, so it fails instead of prompting.
fn check_trusted(req: &Request, path: &Path, trust: &TrustStore) -> Result<(), String> {
    let scripts = &req.scripts;
    let texts = [&scripts.pre_script, &scripts.post_script, &scripts.tests];
    if trust
        .untrusted(texts.into_iter().flatten().map(String::as_str))
        .is_empty()
    {
        return Ok(());
    }
    Err(format!(
        "Untrusted script in {}: open the request and send it once to review and trust it",
        path.display()
    ))
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

        let result = check_trusted(&req, &path, &config.trust).and_then(|()| {
            let exec_req = build_execution_request(&req, &env);
            std::thread::spawn(move || execution::execute(exec_req))
                .join()
                .unwrap_or_else(|_| Err("Request thread panicked".to_string()))
        });

        // Carry forward env changes
        if let Ok(ref res) = result {
            for (k, v) in &res.env_changes {
                env.insert(k.clone(), v.clone());
            }
            for (k, v) in &res.extracted_vars {
                env.insert(k.clone(), v.clone());
            }
        }

        let failed = result
            .as_ref()
            .map(|r| r.test_results.iter().any(|t| !t.passed))
            .unwrap_or(true);

        let run_result = RequestRunResult { name, path, result };
        let _ = tx.send_blocking(RunProgress::Completed {
            index,
            result: run_result,
        });

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
        (
            !path.is_dir(),
            e.file_name().to_string_lossy().to_lowercase(),
        )
    });

    for entry in entries {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            collect_dir(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("http")
            && let Ok(content) = std::fs::read_to_string(&path)
            && let Ok(requests) = http_parser::parse(&content)
        {
            for (i, req) in requests.into_iter().enumerate() {
                let req_name = req.meta.name.clone().unwrap_or_else(|| {
                    let stem = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("request");
                    if i == 0 {
                        stem.to_string()
                    } else {
                        format!("{} #{}", stem, i + 1)
                    }
                });
                out.push((req_name, path.clone(), req));
            }
        }
    }
}

/// Build an ExecutionRequest from a parsed http_parser::Request + current env vars.
fn build_execution_request(req: &Request, env: &HashMap<String, String>) -> ExecutionRequest {
    let sub = |s: &str| substitute(s, env);

    let url = sub(&req.url);
    let headers: Vec<(String, String)> = req
        .headers
        .iter()
        .filter(|h| h.enabled)
        .map(|h| (h.key.clone(), sub(&h.value)))
        .collect();

    let body_str = req.body.as_deref().map(sub).unwrap_or_default();

    let (body, mode) = if req.protocol() == Protocol::GraphQL {
        // Body is JSON like {"query": "...", "variables": {...}}
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body_str) {
            let query = json["query"].as_str().unwrap_or("").to_string();
            let vars = json
                .get("variables")
                .map(|v| serde_json::to_string(v).unwrap_or_default())
                .unwrap_or_default();
            let op = json["operationName"].as_str().map(|s| s.to_string());
            (
                ExecutionBody::None,
                ExecutionMode::GraphQL {
                    query,
                    variables: vars,
                    operation_name: op,
                },
            )
        } else {
            (
                ExecutionBody::Text(body_str),
                ExecutionMode::GraphQL {
                    query: String::new(),
                    variables: String::new(),
                    operation_name: None,
                },
            )
        }
    } else {
        let eb = if body_str.is_empty() {
            ExecutionBody::None
        } else {
            ExecutionBody::Text(body_str)
        };
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
        impersonate_browser: false,
    }
}

/// Substitute `{{var}}` in `input` using `env`.
///
/// Delegates to `Environment::substitute`, which performs a single left-to-right
/// scan rather than N sequential whole-string replaces - this avoids cascading
/// substitution when a variable's value itself contains `{{...}}`-shaped text
/// (e.g. a value captured via `@set` from a prior response in the chain).
pub(crate) fn substitute(input: &str, env: &HashMap<String, String>) -> String {
    let mut environment = crate::models::Environment::new(String::new());
    environment.variables = env.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    environment.substitute(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    #[test]
    fn test_substitute() {
        let mut env = HashMap::new();
        env.insert(
            "base_url".to_string(),
            "https://api.example.com".to_string(),
        );
        env.insert("token".to_string(), "abc123".to_string());
        assert_eq!(
            substitute("{{base_url}}/users", &env),
            "https://api.example.com/users"
        );
        assert_eq!(substitute("Bearer {{token}}", &env), "Bearer abc123");
        assert_eq!(substitute("no vars here", &env), "no vars here");
    }

    /// Regression test: `substitute()` must perform substitution in a single
    /// pass over the original string rather than N sequential whole-string
    /// `.replace()` calls. If a chained variable's value (e.g. one set via
    /// `@set` from a prior response in the chain, such as an API-returned
    /// template string) itself contains literal `{{other_var}}`-shaped text,
    /// that text must be used verbatim and must NOT be expanded again by a
    /// later key's pass. Previously this depended on `HashMap` iteration
    /// order (which key's pass ran "later"), making it a real
    /// chain-consumer-visible bug: a request later in the chain could end up
    /// sending a corrupted/leaked value instead of the literal extracted
    /// string.
    #[test]
    fn test_substitute_does_not_cascade_into_value_containing_variable_syntax() {
        let mut env = HashMap::new();
        env.insert("secret".to_string(), "LEAKED".to_string());
        env.insert("token".to_string(), "{{secret}}".to_string());

        let result = substitute("Authorization: Bearer {{token}}", &env);
        // {{token}}'s literal value "{{secret}}" must be used verbatim, not
        // re-expanded into the "secret" variable's value.
        assert_eq!(result, "Authorization: Bearer {{secret}}");
    }

    #[test]
    fn test_collect_requests_empty_dir() {
        let tmp = TempDir::new("protide_runner_test");
        assert!(collect_requests(tmp.path()).is_empty());
    }

    #[test]
    fn test_collect_requests_finds_http_files() {
        let tmp = TempDir::new("protide_runner_test");
        tmp.write("req.http", b"# @name myReq\nGET https://example.com\n");

        let requests = collect_requests(tmp.path());
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].0, "myReq");
    }

    /// Run `dir` with `trust` and return each request's result. The URL is
    /// a closed loopback port, so a request that does get sent fails fast
    /// without touching the network.
    fn run(dir: &Path, trust: TrustStore) -> Vec<Result<ExecutionResult, String>> {
        let (tx, rx) = async_channel::unbounded();
        let config = RunConfig {
            collection_path: dir.to_path_buf(),
            env_vars: HashMap::new(),
            stop_on_failure: false,
            trust,
        };
        run_collection(config, tx);
        std::iter::from_fn(|| rx.try_recv().ok())
            .filter_map(|event| match event {
                RunProgress::Completed { result, .. } => Some(result.result),
                _ => None,
            })
            .collect()
    }

    const SCRIPTED: &str = "GET http://127.0.0.1:1/\n\n# @pre-script\nenv.set('ran', 'yes');\n";

    #[test]
    fn an_untrusted_script_refuses_its_request_naming_the_file() {
        let tmp = TempDir::new("protide_runner_untrusted");
        let file = tmp.write("shared.http", SCRIPTED.as_bytes());

        let results = run(tmp.path(), TrustStore::default());
        let err = results[0].as_ref().unwrap_err();
        assert!(err.contains("Untrusted script"), "{err}");
        assert!(err.contains(&file.display().to_string()), "{err}");
    }

    #[test]
    fn a_trusted_script_lets_its_request_run() {
        let tmp = TempDir::new("protide_runner_trusted");
        tmp.write("shared.http", SCRIPTED.as_bytes());
        let script = collect_requests(tmp.path())[0].2.scripts.pre_script.clone();
        let mut trust = TrustStore::default();
        trust.trust(&script.unwrap()).unwrap();

        let results = run(tmp.path(), trust);
        if let Err(err) = &results[0] {
            assert!(!err.contains("Untrusted script"), "{err}");
        }
    }
}
