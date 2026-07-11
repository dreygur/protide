//! Extract script results from the JS `__storage` object back into Rust.

use rquickjs::{Ctx, Object};

use crate::scripting::context::ScriptContext;
use crate::scripting::results::{ScriptError, ScriptOutcome, TestResult};

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

    // Env removals
    let removals_arr: rquickjs::Array = storage
        .get("envRemovals")
        .map_err(|e| ScriptError::new(format!("{}", e)))?;
    for i in 0..removals_arr.len() {
        if let Ok(key) = removals_arr.get::<String>(i) {
            script_ctx.remove_env(&key);
        }
    }

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
