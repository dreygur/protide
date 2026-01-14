//! Environment model for variable substitution

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// An environment containing variables
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Environment {
    /// Environment name
    pub name: String,
    /// Variables in this environment
    pub variables: HashMap<String, String>,
    /// Path to the environment file
    #[serde(skip)]
    pub file_path: Option<PathBuf>,
}

impl Environment {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            variables: HashMap::new(),
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
        self.variables.remove(key);
    }

    /// Substitute variables in a string
    /// Variables are in the format {{variable_name}}
    pub fn substitute(&self, input: &str) -> String {
        let mut result = input.to_string();
        for (key, value) in &self.variables {
            let pattern = format!("{{{{{}}}}}", key);
            result = result.replace(&pattern, value);
        }
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
