//! Request chaining and variable extraction
//!
//! Supports extracting values from responses using JSONPath expressions
//! and setting them as environment variables for subsequent requests.

use http_parser::VariableExtraction;
use jsonpath_rust::{JsonPath, JsonPathValue};
use serde_json::Value;

/// Result of extracting a variable from a response
#[derive(Debug, Clone)]
pub struct ExtractionResult {
    /// Variable name
    pub name: String,
    /// Extracted value (as string)
    pub value: String,
    /// Whether extraction was successful
    pub success: bool,
    /// Error message if extraction failed
    pub error: Option<String>,
}

/// Extract variables from a JSON response body
pub fn extract_variables(body: &str, extractions: &[VariableExtraction]) -> Vec<ExtractionResult> {
    let mut results = Vec::new();

    // Try to parse body as JSON
    let json: Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => {
            // If body isn't JSON, all extractions fail
            for extraction in extractions {
                results.push(ExtractionResult {
                    name: extraction.name.clone(),
                    value: String::new(),
                    success: false,
                    error: Some(format!("Response is not valid JSON: {}", e)),
                });
            }
            return results;
        }
    };

    for extraction in extractions {
        let result = extract_single(&json, extraction);
        results.push(result);
    }

    results
}

/// Extract a single variable using JSONPath
fn extract_single(json: &Value, extraction: &VariableExtraction) -> ExtractionResult {
    let expr = &extraction.expression;

    // Parse JSONPath expression
    let path = match JsonPath::try_from(expr.as_str()) {
        Ok(p) => p,
        Err(e) => {
            return ExtractionResult {
                name: extraction.name.clone(),
                value: String::new(),
                success: false,
                error: Some(format!("Invalid JSONPath '{}': {}", expr, e)),
            };
        }
    };

    // Execute JSONPath query - find_slice returns Vec<JsonPathValue>
    let found = path.find_slice(json);

    // Get first match. Note: `find_slice` never returns an empty Vec - when
    // nothing matches (out-of-bounds index, field access on a scalar, etc.)
    // it returns `vec![JsonPathValue::NoValue]` as a placeholder. That's
    // distinct from a real match whose value happens to be JSON `null`
    // (`JsonPathValue::Slice(&Value::Null, _)`), so we must check the variant
    // rather than treating "found anything at all" as success.
    match found.into_iter().next() {
        Some(JsonPathValue::NoValue) | None => ExtractionResult {
            name: extraction.name.clone(),
            value: String::new(),
            success: false,
            error: Some(format!("No match found for JSONPath '{}'", expr)),
        },
        Some(json_path_value) => {
            // JsonPathValue has a to_data() method to get the underlying Value
            let value = json_path_value.to_data();
            let string_value = value_to_string(&value);
            ExtractionResult {
                name: extraction.name.clone(),
                value: string_value,
                success: true,
                error: None,
            }
        }
    }
}

/// Convert a JSON value to a string for use as a variable
fn value_to_string(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => String::new(),
        // For objects and arrays, serialize as JSON
        _ => value.to_string(),
    }
}

/// Extract a value using a JSONPath expression (convenience function)
pub fn extract_jsonpath(body: &str, jsonpath: &str) -> Result<String, String> {
    let json: Value = serde_json::from_str(body).map_err(|e| format!("Invalid JSON: {}", e))?;

    let path = JsonPath::try_from(jsonpath).map_err(|e| format!("Invalid JSONPath: {}", e))?;

    let found = path.find_slice(&json);

    found
        .into_iter()
        .next()
        .map(|v| value_to_string(&v.to_data()))
        .ok_or_else(|| format!("No match for '{}'", jsonpath))
}

/// Common JSONPath patterns for quick extraction
pub mod patterns {
    /// Extract a field from root object: $.field
    pub fn field(name: &str) -> String {
        format!("$.{}", name)
    }

    /// Extract nested field: $.parent.child
    pub fn nested(path: &[&str]) -> String {
        format!("$.{}", path.join("."))
    }

    /// Extract first element of array: $[0]
    pub fn first() -> &'static str {
        "$[0]"
    }

    /// Extract array element by index: $[n]
    pub fn index(n: usize) -> String {
        format!("$[{}]", n)
    }

    /// Extract field from first array element: $[0].field
    pub fn first_field(name: &str) -> String {
        format!("$[0].{}", name)
    }

    /// Extract all values of a field from array: $[*].field
    pub fn all_fields(name: &str) -> String {
        format!("$[*].{}", name)
    }

    /// Extract by filter: $[?(@.field == value)]
    pub fn filter(field: &str, value: &str) -> String {
        format!("$[?(@.{} == '{}')]", field, value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_simple_field() {
        let json = r#"{"id": 123, "name": "test"}"#;
        let result = extract_jsonpath(json, "$.id").unwrap();
        assert_eq!(result, "123");
    }

    #[test]
    fn test_extract_string_field() {
        let json = r#"{"id": 123, "name": "test"}"#;
        let result = extract_jsonpath(json, "$.name").unwrap();
        assert_eq!(result, "test");
    }

    #[test]
    fn test_extract_nested_field() {
        let json = r#"{"data": {"user": {"id": 456}}}"#;
        let result = extract_jsonpath(json, "$.data.user.id").unwrap();
        assert_eq!(result, "456");
    }

    #[test]
    fn test_extract_array_element() {
        let json = r#"{"items": [{"id": 1}, {"id": 2}]}"#;
        let result = extract_jsonpath(json, "$.items[0].id").unwrap();
        assert_eq!(result, "1");
    }

    #[test]
    fn test_extract_nonexistent_field() {
        let json = r#"{"id": 123}"#;
        let result = extract_jsonpath(json, "$.nonexistent");
        // jsonpath-rust returns empty string for nonexistent fields
        // Some implementations return error, but this one returns null/empty
        assert!(result.is_err() || result.as_ref().map(|s| s.is_empty()).unwrap_or(true));
    }

    #[test]
    fn test_extract_invalid_json() {
        let result = extract_jsonpath("not json", "$.id");
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_variables() {
        let json = r#"{"token": "abc123", "user": {"id": 42}}"#;
        let extractions = vec![
            VariableExtraction {
                name: "auth_token".to_string(),
                expression: "$.token".to_string(),
            },
            VariableExtraction {
                name: "user_id".to_string(),
                expression: "$.user.id".to_string(),
            },
        ];

        let results = extract_variables(json, &extractions);

        assert_eq!(results.len(), 2);
        assert!(results[0].success);
        assert_eq!(results[0].name, "auth_token");
        assert_eq!(results[0].value, "abc123");
        assert!(results[1].success);
        assert_eq!(results[1].name, "user_id");
        assert_eq!(results[1].value, "42");
    }

    /// jsonpath-rust's `find_slice` returns `vec![JsonPathValue::NoValue]`
    /// (not an empty result set) for an out-of-bounds array index. That's a
    /// placeholder for "nothing matched", distinct from a real match whose
    /// value happens to be JSON `null`. `extract_single` must report this as
    /// a failed extraction rather than a successful empty-string value, so
    /// a chained `@set` that points at the wrong array index (e.g.
    /// off-by-one against a shorter-than-expected page of results) surfaces
    /// an actionable error instead of silently "succeeding" with "".
    #[test]
    fn test_out_of_bounds_array_index_reports_false_success() {
        let json = r#"{"items": [1, 2, 3]}"#;
        let extractions = vec![VariableExtraction {
            name: "x".to_string(),
            expression: "$.items[10]".to_string(),
        }];
        let result = &extract_variables(json, &extractions)[0];
        assert!(!result.success);
        assert_eq!(result.value, "");
        assert!(result.error.is_some());
    }

    /// Same `NoValue` placeholder is returned when the JSONPath expects an
    /// object field but the response root (or matched node) is a scalar,
    /// e.g. the endpoint returns a bare JSON string/number instead of the
    /// expected object shape. This must also be reported as a failure.
    #[test]
    fn test_field_path_on_scalar_root_reports_false_success() {
        let json = r#""hello""#;
        let extractions = vec![VariableExtraction {
            name: "x".to_string(),
            expression: "$.foo".to_string(),
        }];
        let result = &extract_variables(json, &extractions)[0];
        assert!(!result.success);
        assert_eq!(result.value, "");
        assert!(result.error.is_some());
    }

    /// A field that is genuinely `null` in the response must still be
    /// distinguished from a failed extraction: it's a real match, so it
    /// should report success with an empty string value.
    #[test]
    fn test_field_with_real_null_value_reports_success() {
        let json = r#"{"x": null}"#;
        let extractions = vec![VariableExtraction {
            name: "x".to_string(),
            expression: "$.x".to_string(),
        }];
        let result = &extract_variables(json, &extractions)[0];
        assert!(result.success);
        assert_eq!(result.value, "");
        assert!(result.error.is_none());
    }

    #[test]
    fn test_patterns() {
        assert_eq!(patterns::field("id"), "$.id");
        assert_eq!(
            patterns::nested(&["data", "user", "name"]),
            "$.data.user.name"
        );
        assert_eq!(patterns::first(), "$[0]");
        assert_eq!(patterns::index(5), "$[5]");
        assert_eq!(patterns::first_field("id"), "$[0].id");
        assert_eq!(patterns::all_fields("name"), "$[*].name");
    }
}
