//! JavaScript runtime wrapper using rquickjs

use rquickjs::{Context, Ctx, Function, Object, Runtime, Value};

use super::context::ScriptContext;
use super::results::{ScriptError, ScriptOutcome, TestResult};

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

    /// Construct with a custom deadline — intended for tests only.
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

/// Set up global storage object for collecting results
fn setup_storage(ctx: &Ctx<'_>) -> Result<(), ScriptError> {
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

/// Set up console object using pure JS
fn setup_console_js(ctx: &Ctx<'_>) -> Result<(), ScriptError> {
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

/// Set up env object
fn setup_env_js(
    ctx: &Ctx<'_>,
    env: &std::collections::HashMap<String, String>,
) -> Result<(), ScriptError> {
    // Initialize env data
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

/// Set up request object
fn setup_request_js(
    ctx: &Ctx<'_>,
    request: &super::context::RequestData,
    script_type: ScriptType,
) -> Result<(), ScriptError> {
    // Set request data
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

    // Add mutation methods for pre-script only
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

/// Set up response object
fn setup_response_js(ctx: &Ctx<'_>, response: &super::context::ResponseData) -> Result<(), ScriptError> {
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

    // Add json() method
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

/// Set up expect() function for assertions
fn setup_expect_js(ctx: &Ctx<'_>) -> Result<(), ScriptError> {
    let expect_js = r#"
function expect(actual) {
    return {
        _actual: actual,
        _negated: false,
        get not() {
            const copy = Object.create(this);
            copy._negated = !this._negated;
            return copy;
        },
        _check(passed, name, expected) {
            const finalPassed = this._negated ? !passed : passed;
            const prefix = this._negated ? "not " : "";
            globalThis.__storage.testResults.push({
                passed: finalPassed,
                name: prefix + name,
                expected: String(expected),
                actual: String(this._actual)
            });
            return finalPassed;
        },
        toBe(expected) {
            return this._check(this._actual === expected, "toBe", expected);
        },
        toEqual(expected) {
            const eq = JSON.stringify(this._actual) === JSON.stringify(expected);
            return this._check(eq, "toEqual", expected);
        },
        toBeTruthy() {
            return this._check(!!this._actual, "toBeTruthy", "truthy");
        },
        toBeFalsy() {
            return this._check(!this._actual, "toBeFalsy", "falsy");
        },
        toBeNull() {
            return this._check(this._actual === null, "toBeNull", "null");
        },
        toBeUndefined() {
            return this._check(this._actual === undefined, "toBeUndefined", "undefined");
        },
        toBeDefined() {
            return this._check(this._actual !== undefined, "toBeDefined", "defined");
        },
        toBeGreaterThan(n) {
            return this._check(this._actual > n, "toBeGreaterThan", n);
        },
        toBeGreaterThanOrEqual(n) {
            return this._check(this._actual >= n, "toBeGreaterThanOrEqual", n);
        },
        toBeLessThan(n) {
            return this._check(this._actual < n, "toBeLessThan", n);
        },
        toBeLessThanOrEqual(n) {
            return this._check(this._actual <= n, "toBeLessThanOrEqual", n);
        },
        toContain(item) {
            let contains = false;
            if (typeof this._actual === 'string') {
                contains = this._actual.includes(item);
            } else if (Array.isArray(this._actual)) {
                contains = this._actual.includes(item);
            }
            return this._check(contains, "toContain", item);
        },
        toHaveLength(n) {
            const len = this._actual?.length;
            return this._check(len === n, "toHaveLength", n);
        },
        toHaveProperty(path, value) {
            const parts = path.split('.');
            let obj = this._actual;
            for (const part of parts) {
                if (obj === null || obj === undefined || !Object.hasOwn(obj, part)) {
                    return this._check(false, "toHaveProperty", path);
                }
                obj = obj[part];
            }
            if (arguments.length === 2) {
                return this._check(obj === value, "toHaveProperty", path + " = " + value);
            }
            return this._check(true, "toHaveProperty", path);
        },
        toMatch(pattern) {
            const re = pattern instanceof RegExp ? pattern : new RegExp(pattern);
            return this._check(re.test(this._actual), "toMatch", pattern);
        }
    };
}
globalThis.expect = expect;
"#;

    ctx.eval::<Value, _>(expect_js)
        .map_err(|e| ScriptError::new(format!("Failed to setup expect: {}", e)))?;

    Ok(())
}

/// Set up utility functions (btoa, atob)
fn setup_utils_js(ctx: &Ctx<'_>) -> Result<(), ScriptError> {
    // Use a native Rust btoa/atob since QuickJS doesn't have them
    let btoa_fn = Function::new(ctx.clone(), |s: String| -> String {
        use std::io::Write;
        let mut buf = Vec::new();
        {
            let mut enc = base64_encoder(&mut buf);
            enc.write_all(s.as_bytes()).ok();
        }
        String::from_utf8(buf).unwrap_or_default()
    })
    .map_err(|e| ScriptError::new(format!("{}", e)))?;
    ctx.globals()
        .set("btoa", btoa_fn)
        .map_err(|e| ScriptError::new(format!("{}", e)))?;

    let atob_fn = Function::new(ctx.clone(), |s: String| -> String {
        base64_decode(&s).unwrap_or_default()
    })
    .map_err(|e| ScriptError::new(format!("{}", e)))?;
    ctx.globals()
        .set("atob", atob_fn)
        .map_err(|e| ScriptError::new(format!("{}", e)))?;

    Ok(())
}

/// Extract results from JS storage back to Rust
fn extract_results(ctx: &Ctx<'_>, script_ctx: &mut ScriptContext) -> Result<ScriptOutcome, ScriptError> {
    let globals = ctx.globals();

    // Get storage object
    let storage: Object = globals
        .get("__storage")
        .map_err(|e| ScriptError::new(format!("Failed to get storage: {}", e)))?;

    // Extract console output
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

    // Extract test results
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

    // Extract env changes
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

    // Extract request modifications
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

// Simple base64 encoder
fn base64_encoder(output: &mut Vec<u8>) -> impl std::io::Write + '_ {
    struct Base64Writer<'a>(&'a mut Vec<u8>);
    impl std::io::Write for Base64Writer<'_> {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            const CHARS: &[u8] =
                b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
            for chunk in buf.chunks(3) {
                let b0 = chunk[0] as usize;
                let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
                let b2 = chunk.get(2).copied().unwrap_or(0) as usize;

                self.0.push(CHARS[b0 >> 2]);
                self.0.push(CHARS[((b0 & 0x03) << 4) | (b1 >> 4)]);
                if chunk.len() > 1 {
                    self.0.push(CHARS[((b1 & 0x0F) << 2) | (b2 >> 6)]);
                } else {
                    self.0.push(b'=');
                }
                if chunk.len() > 2 {
                    self.0.push(CHARS[b2 & 0x3F]);
                } else {
                    self.0.push(b'=');
                }
            }
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    Base64Writer(output)
}

// Simple base64 decoder
fn base64_decode(input: &str) -> Option<String> {
    const DECODE: [i8; 128] = {
        let mut t = [-1i8; 128];
        let mut i = 0u8;
        while i < 26 {
            t[(b'A' + i) as usize] = i as i8;
            i += 1;
        }
        i = 0;
        while i < 26 {
            t[(b'a' + i) as usize] = (26 + i) as i8;
            i += 1;
        }
        i = 0;
        while i < 10 {
            t[(b'0' + i) as usize] = (52 + i) as i8;
            i += 1;
        }
        t[b'+' as usize] = 62;
        t[b'/' as usize] = 63;
        t
    };

    let bytes: Vec<u8> = input
        .bytes()
        .filter(|&b| b != b'=' && (b as usize) < 128 && DECODE[b as usize] >= 0)
        .collect();

    let mut output = Vec::new();
    for chunk in bytes.chunks(4) {
        if chunk.len() < 2 {
            break;
        }
        let b0 = DECODE[chunk[0] as usize] as u8;
        let b1 = DECODE[chunk[1] as usize] as u8;
        output.push((b0 << 2) | (b1 >> 4));

        if chunk.len() > 2 {
            let b2 = DECODE[chunk[2] as usize] as u8;
            output.push((b1 << 4) | (b2 >> 2));

            if chunk.len() > 3 {
                let b3 = DECODE[chunk[3] as usize] as u8;
                output.push((b2 << 6) | b3);
            }
        }
    }

    String::from_utf8(output).ok()
}
