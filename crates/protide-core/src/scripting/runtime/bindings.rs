//! JS binding helpers: set up globals and extract results from QuickJS context

pub(super) use super::expect_js::setup_expect_js;

use rquickjs::{Ctx, Function, Object, Value};

use crate::scripting::context::{RequestData, ResponseData, ScriptContext};
use crate::scripting::results::{ScriptError, ScriptOutcome, TestResult};
use super::ScriptType;

/// Set up global storage object for collecting results.
pub(super) fn setup_storage(ctx: &Ctx<'_>) -> Result<(), ScriptError> {
    let init_js = r#"
        globalThis.__storage = {
            consoleOutput: [],
            testResults: [],
            envChanges: [],
            requestMods: {
                url: null,
                headersToSet: [],
                headersToRemove: [],
                body: null
            }
        };
    "#;
    ctx.eval::<Value, _>(init_js)
        .map_err(|e| ScriptError::new(format!("Failed to setup storage: {}", e)))?;
    Ok(())
}

/// Set up `console` object using pure JS.
pub(super) fn setup_console_js(ctx: &Ctx<'_>) -> Result<(), ScriptError> {
    let console_js = r#"
        const console = {
            log(...args) {
                globalThis.__storage.consoleOutput.push(args.map(String).join(' '));
            },
            info(...args) {
                this.log(...args);
            },
            warn(...args) {
                globalThis.__storage.consoleOutput.push('[WARN] ' + args.map(String).join(' '));
            },
            error(...args) {
                globalThis.__storage.consoleOutput.push('[ERROR] ' + args.map(String).join(' '));
            }
        };
        globalThis.console = console;
    "#;
    ctx.eval::<Value, _>(console_js)
        .map_err(|e| ScriptError::new(format!("Failed to setup console: {}", e)))?;
    Ok(())
}

/// Set up `env` object from a Rust HashMap.
pub(super) fn setup_env_js(
    ctx: &Ctx<'_>,
    env: &std::collections::HashMap<String, String>,
) -> Result<(), ScriptError> {
    let env_obj = Object::new(ctx.clone()).map_err(|e| ScriptError::new(format!("{}", e)))?;
    for (k, v) in env {
        env_obj
            .set(k.as_str(), v.clone())
            .map_err(|e| ScriptError::new(format!("{}", e)))?;
    }
    ctx.globals()
        .set("__envData", env_obj)
        .map_err(|e| ScriptError::new(format!("{}", e)))?;

    let env_js = r#"
        const env = {
            get(name) {
                return globalThis.__envData[name] || null;
            },
            set(name, value) {
                globalThis.__envData[name] = value;
                globalThis.__storage.envChanges.push([name, value]);
            },
            has(name) {
                return name in globalThis.__envData;
            },
            remove(name) {
                delete globalThis.__envData[name];
            }
        };
        globalThis.env = env;
    "#;
    ctx.eval::<Value, _>(env_js)
        .map_err(|e| ScriptError::new(format!("Failed to setup env: {}", e)))?;
    Ok(())
}

/// Set up `request` object; adds mutation methods for pre-request scripts.
pub(super) fn setup_request_js(
    ctx: &Ctx<'_>,
    request: &RequestData,
    script_type: ScriptType,
) -> Result<(), ScriptError> {
    let req_obj = Object::new(ctx.clone()).map_err(|e| ScriptError::new(format!("{}", e)))?;
    req_obj
        .set("method", request.method.clone())
        .map_err(|e| ScriptError::new(format!("{}", e)))?;
    req_obj
        .set("url", request.url.clone())
        .map_err(|e| ScriptError::new(format!("{}", e)))?;

    let headers = Object::new(ctx.clone()).map_err(|e| ScriptError::new(format!("{}", e)))?;
    for (k, v) in &request.headers {
        headers
            .set(k.as_str(), v.clone())
            .map_err(|e| ScriptError::new(format!("{}", e)))?;
    }
    req_obj
        .set("headers", headers)
        .map_err(|e| ScriptError::new(format!("{}", e)))?;

    if let Some(ref body) = request.body {
        req_obj
            .set("body", body.clone())
            .map_err(|e| ScriptError::new(format!("{}", e)))?;
    } else {
        req_obj
            .set("body", Value::new_null(ctx.clone()))
            .map_err(|e| ScriptError::new(format!("{}", e)))?;
    }

    ctx.globals()
        .set("request", req_obj)
        .map_err(|e| ScriptError::new(format!("{}", e)))?;

    if matches!(script_type, ScriptType::PreRequest) {
        let mutation_js = r#"
            request.setUrl = function(url) {
                this.url = url;
                globalThis.__storage.requestMods.url = url;
            };
            request.setHeader = function(name, value) {
                this.headers[name] = value;
                globalThis.__storage.requestMods.headersToSet.push([name, value]);
            };
            request.removeHeader = function(name) {
                delete this.headers[name];
                globalThis.__storage.requestMods.headersToRemove.push(name);
            };
            request.setBody = function(body) {
                this.body = body;
                globalThis.__storage.requestMods.body = body;
            };
        "#;
        ctx.eval::<Value, _>(mutation_js)
            .map_err(|e| ScriptError::new(format!("Failed to setup request mutations: {}", e)))?;
    }

    Ok(())
}

/// Set up `response` object with data and helper methods.
pub(super) fn setup_response_js(ctx: &Ctx<'_>, response: &ResponseData) -> Result<(), ScriptError> {
    let resp_obj = Object::new(ctx.clone()).map_err(|e| ScriptError::new(format!("{}", e)))?;

    resp_obj
        .set("status", response.status as i32)
        .map_err(|e| ScriptError::new(format!("{}", e)))?;
    resp_obj
        .set("statusText", response.status_text.clone())
        .map_err(|e| ScriptError::new(format!("{}", e)))?;
    resp_obj
        .set("body", response.body.clone())
        .map_err(|e| ScriptError::new(format!("{}", e)))?;
    resp_obj
        .set("time", response.time_ms as i64)
        .map_err(|e| ScriptError::new(format!("{}", e)))?;
    resp_obj
        .set("size", response.size as i64)
        .map_err(|e| ScriptError::new(format!("{}", e)))?;

    let headers = Object::new(ctx.clone()).map_err(|e| ScriptError::new(format!("{}", e)))?;
    for (k, v) in &response.headers {
        headers
            .set(k.as_str(), v.clone())
            .map_err(|e| ScriptError::new(format!("{}", e)))?;
    }
    resp_obj
        .set("headers", headers)
        .map_err(|e| ScriptError::new(format!("{}", e)))?;

    ctx.globals()
        .set("response", resp_obj)
        .map_err(|e| ScriptError::new(format!("{}", e)))?;

    let json_js = r#"
        response.json = function() {
            return JSON.parse(this.body);
        };
        response.getHeader = function(name) {
            return this.headers[name.toLowerCase()] || null;
        };
    "#;
    ctx.eval::<Value, _>(json_js)
        .map_err(|e| ScriptError::new(format!("Failed to setup response methods: {}", e)))?;

    Ok(())
}

/// Set up `btoa`/`atob` utility functions via native Rust implementations.
pub(super) fn setup_utils_js(ctx: &Ctx<'_>) -> Result<(), ScriptError> {
    let btoa_fn = Function::new(ctx.clone(), |s: String| -> String {
        use std::io::Write;
        let mut buf = Vec::new();
        {
            let mut enc = super::base64::base64_encoder(&mut buf);
            enc.write_all(s.as_bytes()).ok();
        }
        String::from_utf8(buf).unwrap_or_default()
    })
    .map_err(|e| ScriptError::new(format!("{}", e)))?;
    ctx.globals()
        .set("btoa", btoa_fn)
        .map_err(|e| ScriptError::new(format!("{}", e)))?;

    let atob_fn = Function::new(ctx.clone(), |s: String| -> String {
        super::base64::base64_decode(&s).unwrap_or_default()
    })
    .map_err(|e| ScriptError::new(format!("{}", e)))?;
    ctx.globals()
        .set("atob", atob_fn)
        .map_err(|e| ScriptError::new(format!("{}", e)))?;

    Ok(())
}

/// Extract results from JS `__storage` back into the Rust `ScriptContext`.
pub(super) fn extract_results(
    ctx: &Ctx<'_>,
    script_ctx: &mut ScriptContext,
) -> Result<ScriptOutcome, ScriptError> {
    let globals = ctx.globals();

    let storage: Object = globals
        .get("__storage")
        .map_err(|e| ScriptError::new(format!("Failed to get storage: {}", e)))?;

    // Console output
    let console_arr: rquickjs::Array = storage
        .get("consoleOutput")
        .map_err(|e| ScriptError::new(format!("{}", e)))?;
    let mut console_output = Vec::new();
    for i in 0..console_arr.len() {
        if let Ok(s) = console_arr.get::<String>(i) {
            console_output.push(s);
        }
    }
    script_ctx.console_output = console_output.clone();

    // Test results
    let test_arr: rquickjs::Array = storage
        .get("testResults")
        .map_err(|e| ScriptError::new(format!("{}", e)))?;
    let mut test_results = Vec::new();
    for i in 0..test_arr.len() {
        if let Ok(obj) = test_arr.get::<Object>(i) {
            let passed: bool = obj.get("passed").unwrap_or(false);
            let name: String = obj.get("name").unwrap_or_default();
            let expected: String = obj.get("expected").unwrap_or_default();
            let actual: String = obj.get("actual").unwrap_or_default();
            if passed {
                test_results.push(TestResult::pass(&name));
            } else {
                test_results.push(TestResult::fail(&name, &expected, &actual));
            }
        }
    }
    script_ctx.test_results = test_results.clone();

    // Env changes
    let env_arr: rquickjs::Array = storage
        .get("envChanges")
        .map_err(|e| ScriptError::new(format!("{}", e)))?;
    let mut env_changes = Vec::new();
    for i in 0..env_arr.len() {
        if let Ok(pair) = env_arr.get::<rquickjs::Array>(i) {
            let key: String = pair.get(0).unwrap_or_default();
            let value: String = pair.get(1).unwrap_or_default();
            env_changes.push((key.clone(), value.clone()));
            script_ctx.env.insert(key, value);
        }
    }
    script_ctx.env_changes = env_changes.clone();

    // Request modifications
    let mods: Object = storage
        .get("requestMods")
        .map_err(|e| ScriptError::new(format!("{}", e)))?;

    let url: Option<String> = mods.get("url").ok();
    script_ctx.modified_request.url = url.clone();

    let headers_arr: rquickjs::Array = mods.get("headersToSet").unwrap_or_else(|_| {
        rquickjs::Array::new(ctx.clone()).unwrap()
    });
    let mut headers_to_set = Vec::new();
    for i in 0..headers_arr.len() {
        if let Ok(pair) = headers_arr.get::<rquickjs::Array>(i) {
            let key: String = pair.get(0).unwrap_or_default();
            let value: String = pair.get(1).unwrap_or_default();
            headers_to_set.push((key, value));
        }
    }
    script_ctx.modified_request.headers_to_set = headers_to_set;

    let remove_arr: rquickjs::Array = mods.get("headersToRemove").unwrap_or_else(|_| {
        rquickjs::Array::new(ctx.clone()).unwrap()
    });
    let mut headers_to_remove = Vec::new();
    for i in 0..remove_arr.len() {
        if let Ok(s) = remove_arr.get::<String>(i) {
            headers_to_remove.push(s);
        }
    }
    script_ctx.modified_request.headers_to_remove = headers_to_remove;

    let body: Option<String> = mods.get("body").ok();
    script_ctx.modified_request.body = body;

    Ok(ScriptOutcome {
        success: true,
        error: None,
        test_results,
        console_output,
        modified_request: script_ctx.modified_request.clone(),
        env_changes,
    })
}
