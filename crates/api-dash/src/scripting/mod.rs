//! JavaScript scripting engine for API Dash
//!
//! Provides pre-request scripts, post-response scripts, and test assertions.
//!
//! # Example
//!
//! ```ignore
//! let engine = ScriptEngine::new()?;
//! let mut ctx = ScriptContext::new()
//!     .with_request(RequestData::new("GET", "https://api.example.com"))
//!     .with_env(env_vars);
//!
//! // Run pre-request script
//! let outcome = engine.run_pre_script("request.setHeader('X-Test', 'value')", &mut ctx)?;
//!
//! // After HTTP request, run tests
//! ctx.set_response(response_data);
//! let outcome = engine.run_tests("expect(response.status).toBe(200)", &mut ctx)?;
//! ```

pub mod context;
pub mod results;
mod runtime;

pub use context::{RequestData, ResponseData, ScriptContext};
pub use results::{ScriptError, ScriptOutcome};
use runtime::{JsRuntime, ScriptType};

/// JavaScript scripting engine
pub struct ScriptEngine {
    runtime: JsRuntime,
}

impl ScriptEngine {
    /// Create a new script engine
    pub fn new() -> Result<Self, ScriptError> {
        Ok(Self {
            runtime: JsRuntime::new()?,
        })
    }

    /// Execute a pre-request script
    ///
    /// Pre-scripts can modify the request using:
    /// - `request.setUrl(url)`
    /// - `request.setHeader(name, value)`
    /// - `request.removeHeader(name)`
    /// - `request.setBody(body)`
    /// - `env.set(name, value)`
    pub fn run_pre_script(
        &self,
        script: &str,
        ctx: &mut ScriptContext,
    ) -> Result<ScriptOutcome, ScriptError> {
        if script.trim().is_empty() {
            return Ok(ScriptOutcome::success());
        }
        self.runtime.execute(script, ctx, ScriptType::PreRequest)
    }

    /// Execute a post-response script
    ///
    /// Post-scripts can access the response and set environment variables:
    /// - `response.status`, `response.body`, `response.json()`, etc.
    /// - `env.set(name, value)` to save values for later requests
    pub fn run_post_script(
        &self,
        script: &str,
        ctx: &mut ScriptContext,
    ) -> Result<ScriptOutcome, ScriptError> {
        if script.trim().is_empty() {
            return Ok(ScriptOutcome::success());
        }
        self.runtime.execute(script, ctx, ScriptType::PostResponse)
    }

    /// Execute test assertions
    ///
    /// Tests use the `expect()` API:
    /// - `expect(response.status).toBe(200)`
    /// - `expect(response.body).toContain("success")`
    /// - `expect(response.time).toBeLessThan(1000)`
    pub fn run_tests(
        &self,
        script: &str,
        ctx: &mut ScriptContext,
    ) -> Result<ScriptOutcome, ScriptError> {
        if script.trim().is_empty() {
            return Ok(ScriptOutcome::success());
        }
        self.runtime.execute(script, ctx, ScriptType::Tests)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_engine_creation() {
        let engine = ScriptEngine::new();
        assert!(engine.is_ok());
    }

    #[test]
    fn test_empty_script() {
        let engine = ScriptEngine::new().unwrap();
        let mut ctx = ScriptContext::new();
        let result = engine.run_pre_script("", &mut ctx);
        assert!(result.is_ok());
        assert!(result.unwrap().success);
    }

    #[test]
    fn test_console_log() {
        let engine = ScriptEngine::new().unwrap();
        let mut ctx = ScriptContext::new();
        let result = engine.run_pre_script("console.log('hello', 'world')", &mut ctx);
        assert!(result.is_ok());
        let outcome = result.unwrap();
        assert!(outcome.success);
        assert_eq!(outcome.console_output, vec!["hello world"]);
    }

    #[test]
    fn test_env_get_set() {
        let engine = ScriptEngine::new().unwrap();
        let mut env = HashMap::new();
        env.insert("existing".to_string(), "value".to_string());
        let mut ctx = ScriptContext::new().with_env(env);

        let script = r#"
            console.log(env.get('existing'));
            env.set('new_var', 'new_value');
        "#;

        let result = engine.run_pre_script(script, &mut ctx);
        assert!(result.is_ok());
        let outcome = result.unwrap();
        assert!(outcome.success);
        assert_eq!(outcome.console_output, vec!["value"]);
        assert!(outcome.env_changes.contains(&("new_var".to_string(), "new_value".to_string())));
    }

    #[test]
    fn test_request_modification() {
        let engine = ScriptEngine::new().unwrap();
        let request = RequestData::new("GET", "https://example.com");
        let mut ctx = ScriptContext::new().with_request(request);

        let script = r#"
            request.setHeader('X-Custom', 'test-value');
            request.setUrl('https://modified.com');
        "#;

        let result = engine.run_pre_script(script, &mut ctx);
        assert!(result.is_ok());
        let outcome = result.unwrap();
        assert!(outcome.success);
        assert_eq!(outcome.modified_request.url, Some("https://modified.com".to_string()));
        assert!(outcome.modified_request.headers_to_set.contains(&("X-Custom".to_string(), "test-value".to_string())));
    }

    #[test]
    fn test_response_access() {
        let engine = ScriptEngine::new().unwrap();
        let mut ctx = ScriptContext::new();
        ctx.set_response(ResponseData::new(200, "OK", r#"{"message":"success"}"#.to_string()));

        let script = r#"
            console.log(response.status);
            console.log(response.statusText);
        "#;

        let result = engine.run_post_script(script, &mut ctx);
        assert!(result.is_ok());
        let outcome = result.unwrap();
        assert!(outcome.success);
        assert!(outcome.console_output.contains(&"200".to_string()));
        assert!(outcome.console_output.contains(&"OK".to_string()));
    }

    #[test]
    fn test_expect_to_be() {
        let engine = ScriptEngine::new().unwrap();
        let mut ctx = ScriptContext::new();
        ctx.set_response(ResponseData::new(200, "OK", "{}".to_string()));

        let script = r#"
            expect(response.status).toBe(200);
            expect(response.status).toBe(404);
        "#;

        let result = engine.run_tests(script, &mut ctx);
        assert!(result.is_ok());
        let outcome = result.unwrap();
        assert!(outcome.success); // Script ran successfully
        assert_eq!(outcome.test_results.len(), 2);
        assert!(outcome.test_results[0].passed); // 200 === 200
        assert!(!outcome.test_results[1].passed); // 200 !== 404
    }

    #[test]
    fn test_expect_not() {
        let engine = ScriptEngine::new().unwrap();
        let mut ctx = ScriptContext::new();
        ctx.set_response(ResponseData::new(200, "OK", "{}".to_string()));

        let script = r#"
            expect(response.status).not.toBe(404);
        "#;

        let result = engine.run_tests(script, &mut ctx);
        assert!(result.is_ok());
        let outcome = result.unwrap();
        assert!(outcome.test_results[0].passed);
    }

    #[test]
    fn test_btoa_atob() {
        let engine = ScriptEngine::new().unwrap();
        let mut ctx = ScriptContext::new();

        let script = r#"
            const encoded = btoa('hello');
            console.log(encoded);
            const decoded = atob(encoded);
            console.log(decoded);
        "#;

        let result = engine.run_pre_script(script, &mut ctx);
        assert!(result.is_ok());
        let outcome = result.unwrap();
        assert!(outcome.success);
        assert_eq!(outcome.console_output[0], "aGVsbG8=");
        assert_eq!(outcome.console_output[1], "hello");
    }
}
