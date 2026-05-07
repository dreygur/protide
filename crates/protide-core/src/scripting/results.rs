//! Script execution result types

/// Result of running a single test assertion
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct TestResult {
    /// Test description (from expect chain)
    pub name: String,
    /// Whether the test passed
    pub passed: bool,
    /// Expected value (for display)
    pub expected: String,
    /// Actual value (for display)
    pub actual: String,
    /// Error message if test failed
    pub error: Option<String>,
}

impl TestResult {
    pub fn pass(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            passed: true,
            ..Default::default()
        }
    }

    pub fn fail(name: impl Into<String>, expected: impl Into<String>, actual: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            passed: false,
            expected: expected.into(),
            actual: actual.into(),
            error: None,
        }
    }

    pub fn error(name: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            passed: false,
            error: Some(error.into()),
            ..Default::default()
        }
    }
}

/// Modifications to request from pre-script
#[derive(Debug, Clone, Default)]
pub struct ModifiedRequest {
    /// Modified URL (if changed)
    pub url: Option<String>,
    /// Headers to set (key, value pairs)
    pub headers_to_set: Vec<(String, String)>,
    /// Headers to remove (by name)
    pub headers_to_remove: Vec<String>,
    /// Modified body (if changed)
    pub body: Option<String>,
}

/// Script execution error
#[derive(Debug, Clone)]
pub struct ScriptError {
    /// Error message
    pub message: String,
    /// Line number where error occurred (if available)
    pub line: Option<usize>,
    /// Stack trace (if available)
    pub stack: Option<String>,
}

impl ScriptError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            line: None,
            stack: None,
        }
    }

    pub fn with_line(mut self, line: usize) -> Self {
        self.line = Some(line);
        self
    }

    pub fn with_stack(mut self, stack: impl Into<String>) -> Self {
        self.stack = Some(stack.into());
        self
    }
}

impl std::fmt::Display for ScriptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)?;
        if let Some(line) = self.line {
            write!(f, " (line {})", line)?;
        }
        Ok(())
    }
}

impl std::error::Error for ScriptError {}

/// Outcome of executing a script
#[derive(Debug, Clone, Default)]
pub struct ScriptOutcome {
    /// Whether script executed successfully
    pub success: bool,
    /// Error if script failed
    pub error: Option<ScriptError>,
    /// Test results (from expect() calls)
    pub test_results: Vec<TestResult>,
    /// Console output (from console.log, etc.)
    pub console_output: Vec<String>,
    /// Request modifications (from pre-script)
    pub modified_request: ModifiedRequest,
    /// Environment variable changes (key, value)
    pub env_changes: Vec<(String, String)>,
}

impl ScriptOutcome {
    pub fn success() -> Self {
        Self {
            success: true,
            ..Default::default()
        }
    }

    pub fn error(err: ScriptError) -> Self {
        Self {
            success: false,
            error: Some(err),
            ..Default::default()
        }
    }
}
