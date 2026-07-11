//! Environment model for variable substitution

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// An environment containing variables
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Environment {
    /// Environment name
    pub name: String,
    /// Variables in this environment
    pub variables: IndexMap<String, String>,
    /// Path to the environment file
    #[serde(skip)]
    pub file_path: Option<PathBuf>,
}

impl Environment {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            variables: IndexMap::new(),
            file_path: None,
        }
    }

    /// Get a variable value
    pub fn get(&self, key: &str) -> Option<&String> {
        self.variables.get(key)
    }

    /// Set a variable value
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.variables.insert(key.into(), value.into());
    }

    /// Remove a variable
    pub fn remove(&mut self, key: &str) {
        self.variables.shift_remove(key);
    }

    /// Substitute variables in a string
    /// Variables are in the format {{variable_name}}
    ///
    /// Performs a single left-to-right scan over `input`, looking up each
    /// `{{name}}` occurrence directly in `variables`. Substituted values are
    /// copied verbatim into the output and never re-scanned, so a value that
    /// itself contains `{{...}}`-shaped text (e.g. one set via `@set` from a
    /// prior response) cannot be expanded a second time, and the result no
    /// longer depends on map iteration order.
    pub fn substitute(&self, input: &str) -> String {
        let mut result = String::with_capacity(input.len());
        let mut rest = input;
        while let Some(start) = rest.find("{{") {
            result.push_str(&rest[..start]);
            let after_open = &rest[start + 2..];
            if let Some(end) = after_open.find("}}") {
                let var_name = &after_open[..end];
                match self.variables.get(var_name) {
                    Some(value) => result.push_str(value),
                    None => result.push_str(&rest[start..start + 2 + end + 2]),
                }
                rest = &after_open[end + 2..];
            } else {
                // No closing `}}` - emit the rest verbatim and stop.
                result.push_str(&rest[start..]);
                rest = "";
                break;
            }
        }
        result.push_str(rest);
        result
    }

    /// Get all variable names referenced in a string (simple parser, no regex)
    pub fn find_variables(input: &str) -> Vec<String> {
        let mut vars = Vec::new();
        let mut chars = input.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '{' && chars.peek() == Some(&'{') {
                chars.next(); // consume second '{'
                let mut var_name = String::new();
                while let Some(c) = chars.next() {
                    if c == '}' && chars.peek() == Some(&'}') {
                        chars.next(); // consume second '}'
                        if !var_name.is_empty() {
                            vars.push(var_name);
                        }
                        break;
                    }
                    var_name.push(c);
                }
            }
        }
        vars
    }

    /// Check if a string contains any variable references
    pub fn has_variables(input: &str) -> bool {
        input.contains("{{") && input.contains("}}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_substitute_basic() {
        let mut env = Environment::new("test");
        env.set("base_url", "https://api.example.com");
        env.set("token", "abc123");
        assert_eq!(env.substitute("{{base_url}}/users"), "https://api.example.com/users");
        assert_eq!(env.substitute("Bearer {{token}}"), "Bearer abc123");
        assert_eq!(env.substitute("no vars here"), "no vars here");
    }

    #[test]
    fn test_substitute_leaves_unknown_variable_untouched() {
        let env = Environment::new("test");
        assert_eq!(env.substitute("{{missing}}"), "{{missing}}");
    }

    /// Regression test: a variable's value that itself looks like a
    /// `{{other_var}}` reference (e.g. a value captured via `@set` from a
    /// prior response in a chain) must be substituted in verbatim and must
    /// NOT be expanded again in the same pass, regardless of which order the
    /// variables happen to be stored/iterated in.
    #[test]
    fn test_substitute_does_not_cascade_into_value_containing_variable_syntax() {
        let mut env = Environment::new("test");
        env.set("secret", "LEAKED");
        env.set("token", "{{secret}}");

        let result = env.substitute("Authorization: Bearer {{token}}");
        // {{token}}'s literal value "{{secret}}" must be used verbatim.
        assert_eq!(result, "Authorization: Bearer {{secret}}");

        // Order-independence: inserting in the opposite order must yield the
        // same result, since a single-pass scan never re-visits substituted
        // output regardless of map iteration/insertion order.
        let mut env2 = Environment::new("test");
        env2.set("token", "{{secret}}");
        env2.set("secret", "LEAKED");
        assert_eq!(env2.substitute("Authorization: Bearer {{token}}"), result);
    }

    #[test]
    fn test_substitute_handles_unclosed_braces() {
        let mut env = Environment::new("test");
        env.set("foo", "bar");
        assert_eq!(env.substitute("prefix {{foo}} mid {{unterminated"), "prefix bar mid {{unterminated");
    }
}

impl Default for Environment {
    fn default() -> Self {
        Self::new("Default")
    }
}

/// Global environment state manager
#[derive(Debug, Clone, Default)]
pub struct EnvironmentState {
    /// All available environments
    pub environments: Vec<Environment>,
    /// Index of the active environment (None = no environment)
    pub active_index: Option<usize>,
}

impl EnvironmentState {
    pub fn new() -> Self {
        // Create default environments
        let mut dev = Environment::new("Development");
        dev.set("base_url", "http://localhost:3000");
        dev.set("api_key", "dev-api-key-123");

        let mut prod = Environment::new("Production");
        prod.set("base_url", "https://api.example.com");
        prod.set("api_key", "prod-api-key-456");

        Self {
            environments: vec![dev, prod],
            active_index: Some(0), // Default to Development
        }
    }

    /// Get the active environment
    pub fn active(&self) -> Option<&Environment> {
        self.active_index.and_then(|i| self.environments.get(i))
    }

    /// Get the active environment mutably
    pub fn active_mut(&mut self) -> Option<&mut Environment> {
        self.active_index.and_then(|i| self.environments.get_mut(i))
    }

    /// Set the active environment by index
    pub fn set_active(&mut self, index: Option<usize>) {
        if let Some(i) = index {
            if i < self.environments.len() {
                self.active_index = Some(i);
            }
        } else {
            self.active_index = None;
        }
    }

    /// Add a new environment
    pub fn add_environment(&mut self, env: Environment) {
        self.environments.push(env);
    }

    /// Remove an environment by index
    pub fn remove_environment(&mut self, index: usize) {
        if index < self.environments.len() && self.environments.len() > 1 {
            self.environments.remove(index);
            // Adjust active index if needed
            if let Some(active) = self.active_index {
                if active == index {
                    self.active_index = Some(0);
                } else if active > index {
                    self.active_index = Some(active - 1);
                }
            }
        }
    }

    /// Substitute variables in a string using the active environment
    pub fn substitute(&self, input: &str) -> String {
        if let Some(env) = self.active() {
            env.substitute(input)
        } else {
            input.to_string()
        }
    }

    /// Get environment names for display
    pub fn environment_names(&self) -> Vec<&str> {
        self.environments.iter().map(|e| e.name.as_str()).collect()
    }
}
