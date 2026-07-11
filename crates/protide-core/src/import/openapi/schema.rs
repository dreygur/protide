use std::collections::HashSet;

use serde_json::Value;

/// Resolve a local JSON $ref like "#/components/schemas/Foo"
pub(super) fn resolve_ref<'a>(root: &'a Value, reference: &str) -> Option<&'a Value> {
    let path = reference.strip_prefix("#/")?;
    let mut current = root;
    for segment in path.split('/') {
        let key = segment.replace("~1", "/").replace("~0", "~");
        current = current.get(&key)?;
    }
    Some(current)
}

/// Get example body string from a content entry (OpenAPI requestBody content value)
pub(super) fn get_schema_example(content: &Value, root: &Value) -> Option<String> {
    if let Some(example) = content.get("example") {
        return Some(if example.is_string() {
            example.as_str().unwrap_or_default().to_string()
        } else {
            serde_json::to_string_pretty(example).ok()?
        });
    }

    if let Some(schema) = content.get("schema") {
        let resolved = if let Some(r) = schema.get("$ref").and_then(|v| v.as_str()) {
            resolve_ref(root, r).unwrap_or(schema)
        } else {
            schema
        };

        if let Some(example) = resolved.get("example") {
            return Some(if example.is_string() {
                example.as_str().unwrap_or_default().to_string()
            } else {
                serde_json::to_string_pretty(example).ok()?
            });
        }

        return generate_schema_example(resolved, root, &mut HashSet::new());
    }

    None
}

/// Get example string for a parameter value
pub(super) fn get_example_value(param: &Value, root: &Value) -> String {
    if let Some(example) = param.get("example") {
        if let Some(s) = example.as_str() {
            return s.to_string();
        }
        return example.to_string();
    }

    if let Some(schema) = param.get("schema") {
        let resolved = if let Some(r) = schema.get("$ref").and_then(|v| v.as_str()) {
            resolve_ref(root, r).unwrap_or(schema)
        } else {
            schema
        };

        if let Some(example) = resolved.get("example") {
            if let Some(s) = example.as_str() {
                return s.to_string();
            }
            return example.to_string();
        }

        let schema_type = resolved.get("type").and_then(|v| v.as_str()).unwrap_or("string");
        return match schema_type {
            "integer" | "number" => "0".to_string(),
            "boolean" => "true".to_string(),
            _ => "{{value}}".to_string(),
        };
    }

    "{{value}}".to_string()
}

/// Generate a pretty-printed JSON example string for a schema.
///
/// `visited` tracks `$ref` names currently being resolved on the current
/// recursion path, so that self-referential schemas (e.g. `Node { children:
/// Node[] }`) don't recurse forever and stack-overflow. When a `$ref`
/// already on the path is encountered, recursion stops for that branch and a
/// minimal placeholder is returned instead.
fn generate_schema_example(schema: &Value, root: &Value, visited: &mut HashSet<String>) -> Option<String> {
    let ref_name = schema.get("$ref").and_then(|v| v.as_str()).map(|s| s.to_string());
    let schema = if let Some(r) = &ref_name {
        resolve_ref(root, r).unwrap_or(schema)
    } else {
        schema
    };

    if let Some(r) = &ref_name
        && !visited.insert(r.clone()) {
            // Cycle detected: this $ref is already being resolved further up
            // the recursion path. Stop here instead of recursing again.
            return Some("{}".to_string());
        }

    let result = generate_schema_example_inner(schema, root, visited);

    if let Some(r) = &ref_name {
        visited.remove(r);
    }

    result
}

fn generate_schema_example_inner(schema: &Value, root: &Value, visited: &mut HashSet<String>) -> Option<String> {
    // allOf: merge properties from all sub-schemas
    if let Some(all_of) = schema.get("allOf").and_then(|v| v.as_array()) {
        let mut merged = serde_json::Map::new();
        for sub in all_of {
            let resolved = if let Some(r) = sub.get("$ref").and_then(|v| v.as_str()) {
                resolve_ref(root, r).unwrap_or(sub)
            } else {
                sub
            };
            if let Some(props) = resolved.get("properties").and_then(|v| v.as_object()) {
                for (k, v) in props {
                    merged.insert(k.clone(), generate_property_example(v, root, visited));
                }
            }
        }
        if !merged.is_empty() {
            return serde_json::to_string_pretty(&Value::Object(merged)).ok();
        }
    }

    let schema_type = schema.get("type").and_then(|v| v.as_str());

    // No explicit type but has properties → treat as object
    if schema_type.is_none() && schema.get("properties").is_some() {
        let mut obj = serde_json::Map::new();
        if let Some(props) = schema.get("properties").and_then(|v| v.as_object()) {
            for (key, prop) in props {
                obj.insert(key.clone(), generate_property_example(prop, root, visited));
            }
        }
        return serde_json::to_string_pretty(&Value::Object(obj)).ok();
    }

    match schema_type? {
        "object" => {
            let mut obj = serde_json::Map::new();
            if let Some(props) = schema.get("properties").and_then(|v| v.as_object()) {
                for (key, prop) in props {
                    obj.insert(key.clone(), generate_property_example(prop, root, visited));
                }
            }
            serde_json::to_string_pretty(&Value::Object(obj)).ok()
        }
        "array" => {
            if let Some(items) = schema.get("items") {
                let item = generate_property_example(items, root, visited);
                serde_json::to_string_pretty(&Value::Array(vec![item])).ok()
            } else {
                Some("[]".to_string())
            }
        }
        "string" => Some("\"string\"".to_string()),
        "integer" | "number" => Some("0".to_string()),
        "boolean" => Some("true".to_string()),
        _ => None,
    }
}

/// Generate an example `Value` for a single property schema. See
/// [`generate_schema_example`] for the cycle-detection contract of `visited`.
fn generate_property_example(schema: &Value, root: &Value, visited: &mut HashSet<String>) -> Value {
    let ref_name = schema.get("$ref").and_then(|v| v.as_str()).map(|s| s.to_string());
    let schema = if let Some(r) = &ref_name {
        resolve_ref(root, r).unwrap_or(schema)
    } else {
        schema
    };

    if let Some(r) = &ref_name
        && !visited.insert(r.clone()) {
            // Cycle detected: stop recursing for this branch.
            return Value::Object(serde_json::Map::new());
        }

    let result = generate_property_example_inner(schema, root, visited);

    if let Some(r) = &ref_name {
        visited.remove(r);
    }

    result
}

fn generate_property_example_inner(schema: &Value, root: &Value, visited: &mut HashSet<String>) -> Value {
    if let Some(example) = schema.get("example") {
        return example.clone();
    }

    let schema_type = schema.get("type").and_then(|v| v.as_str()).unwrap_or("string");

    match schema_type {
        "string" => match schema.get("format").and_then(|v| v.as_str()) {
            Some("email") => Value::String("user@example.com".to_string()),
            Some("uri") | Some("url") => Value::String("https://example.com".to_string()),
            Some("uuid") => Value::String("550e8400-e29b-41d4-a716-446655440000".to_string()),
            Some("date") => Value::String("2024-01-01".to_string()),
            Some("date-time") => Value::String("2024-01-01T00:00:00Z".to_string()),
            _ => Value::String("string".to_string()),
        },
        "integer" => Value::Number(0.into()),
        "number" => serde_json::Number::from_f64(0.0).map(Value::Number).unwrap_or(Value::Null),
        "boolean" => Value::Bool(true),
        "array" => {
            if let Some(items) = schema.get("items") {
                Value::Array(vec![generate_property_example(items, root, visited)])
            } else {
                Value::Array(vec![])
            }
        }
        "object" => {
            let mut obj = serde_json::Map::new();
            if let Some(props) = schema.get("properties").and_then(|v| v.as_object()) {
                for (key, prop) in props {
                    obj.insert(key.clone(), generate_property_example(prop, root, visited));
                }
            }
            Value::Object(obj)
        }
        _ => Value::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// A self-referential schema (`Node { children: Node[] }`) previously
    /// caused unbounded recursion through generate_schema_example ->
    /// generate_property_example -> generate_schema_example... and would
    /// stack-overflow-crash the process. Cycle detection via `visited` must
    /// stop the recursion and return a placeholder instead.
    #[test]
    fn test_self_referential_schema_does_not_stack_overflow() {
        let root = json!({
            "components": {
                "schemas": {
                    "Node": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string" },
                            "children": {
                                "type": "array",
                                "items": { "$ref": "#/components/schemas/Node" }
                            }
                        }
                    }
                }
            }
        });

        let content = json!({
            "schema": { "$ref": "#/components/schemas/Node" }
        });

        // Must return without crashing / infinitely recursing.
        let example = get_schema_example(&content, &root);
        assert!(example.is_some());
        let parsed: Value = serde_json::from_str(&example.unwrap()).unwrap();
        assert!(parsed.is_object());
        assert_eq!(parsed.get("name").and_then(|v| v.as_str()), Some("string"));
        // The cyclic "children" array should contain a terminated
        // placeholder rather than infinitely nested Nodes.
        assert!(parsed.get("children").is_some());
    }

    /// Two schemas that reference each other (A -> B -> A) should also
    /// terminate instead of recursing forever.
    #[test]
    fn test_mutually_referential_schemas_does_not_stack_overflow() {
        let root = json!({
            "components": {
                "schemas": {
                    "A": {
                        "type": "object",
                        "properties": {
                            "b": { "$ref": "#/components/schemas/B" }
                        }
                    },
                    "B": {
                        "type": "object",
                        "properties": {
                            "a": { "$ref": "#/components/schemas/A" }
                        }
                    }
                }
            }
        });

        let content = json!({
            "schema": { "$ref": "#/components/schemas/A" }
        });

        let example = get_schema_example(&content, &root);
        assert!(example.is_some());
        let parsed: Value = serde_json::from_str(&example.unwrap()).unwrap();
        assert!(parsed.is_object());
    }
}
