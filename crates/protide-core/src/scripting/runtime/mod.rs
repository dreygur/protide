//! JavaScript runtime wrapper using rquickjs

mod base64;
mod bindings;
mod expect_js;

use bindings::{
    extract_results, setup_console_js, setup_env_js, setup_expect_js, setup_request_js,
    setup_response_js, setup_storage, setup_utils_js,
};
use rquickjs::{Context, Runtime, Value};

use super::context::ScriptContext;
use super::results::{ScriptError, ScriptOutcome};

/// Default script execution deadline in milliseconds.
const DEFAULT_SCRIPT_TIMEOUT_MS: u64 = 5000;

/// JavaScript runtime wrapper.
/// Creates a fresh QuickJS runtime for each execution to avoid GC issues.
pub struct JsRuntime {
    timeout_ms: u64,
}

impl JsRuntime {
    pub fn new() -> Result<Self, ScriptError> {
        Ok(Self { timeout_ms: DEFAULT_SCRIPT_TIMEOUT_MS })
    }

    /// Construct with a custom deadline - intended for tests only.
    #[cfg(test)]
    pub fn with_timeout_ms(timeout_ms: u64) -> Result<Self, ScriptError> {
        Ok(Self { timeout_ms })
    }

    /// Execute a script with the given context.
    ///
    /// An interrupt handler is installed on the QuickJS runtime that fires once
    /// `timeout_ms` has elapsed. QuickJS checks this handler periodically during
    /// opcode dispatch; a tight infinite loop will be interrupted within a few
    /// milliseconds of the deadline.
    pub fn execute(
        &self,
        script: &str,
        ctx: &mut ScriptContext,
        script_type: ScriptType,
    ) -> Result<ScriptOutcome, ScriptError> {
        let timeout_ms = self.timeout_ms;

        // Create fresh runtime for each execution to avoid GC/state leakage.
        let runtime = Runtime::new()
            .map_err(|e| ScriptError::new(format!("Failed to create JS runtime: {}", e)))?;

        // Install deadline-based interrupt handler. QuickJS calls this closure
        // periodically during script execution. Returning `true` causes the engine
        // to throw an InternalError and unwind the call stack cleanly.
        let deadline = std::time::Instant::now()
            + std::time::Duration::from_millis(timeout_ms);
        runtime.set_interrupt_handler(Some(Box::new(move || {
            std::time::Instant::now() >= deadline
        })));

        let context = Context::full(&runtime)
            .map_err(|e| ScriptError::new(format!("Failed to create JS context: {}", e)))?;

        let result = context.with(|js_ctx| {
            setup_storage(&js_ctx)?;
            setup_console_js(&js_ctx)?;
            setup_env_js(&js_ctx, &ctx.env)?;
            setup_request_js(&js_ctx, &ctx.request, script_type)?;

            if let Some(ref resp) = ctx.response {
                setup_response_js(&js_ctx, resp)?;
            }

            if matches!(script_type, ScriptType::Tests) {
                setup_expect_js(&js_ctx)?;
            }

            setup_utils_js(&js_ctx)?;

            // Distinguish a timeout-induced interrupt from a genuine script error.
            let eval_result = js_ctx.eval::<Value, _>(script);
            eval_result.map_err(|e| {
                let is_timeout = std::time::Instant::now() >= deadline
                    || e.to_string().contains("interrupted");
                if is_timeout {
                    ScriptError::new(format!(
                        "Script execution timed out after {}ms",
                        timeout_ms
                    ))
                } else {
                    ScriptError::new(format!("Script error: {}", e))
                }
            })?;

            extract_results(&js_ctx, ctx)
        });

        runtime.run_gc();
        result
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScriptType {
    PreRequest,
    PostResponse,
    Tests,
}
