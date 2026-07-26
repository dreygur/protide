//! JS binding helpers: set up globals and extract results from QuickJS context

pub(super) use super::expect_js::setup_expect_js;

use rquickjs::{Ctx, Function, Object, Value};

use super::ScriptType;
use crate::scripting::context::{RequestData, ResponseData};
use crate::scripting::results::ScriptError;

/// Set up global storage object for collecting results.
pub(super) fn setup_storage(ctx: &Ctx<'_>) -> Result<(), ScriptError> {
    let init_js = r#"
        globalThis.__storage = {
            consoleOutput: [],
            testResults: [],
            envChanges: [],
            envRemovals: [],
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
                globalThis.__storage.envRemovals.push(name);
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
