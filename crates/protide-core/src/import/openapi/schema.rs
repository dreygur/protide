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
            example.as_str().unwrap().to_string()
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
                example.as_str().unwrap().to_string()
            } else {
                serde_json::to_string_pretty(example).ok()?
            });
        }

        return generate_schema_example(resolved, root);
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

fn generate_schema_example(schema: &Value, root: &Value) -> Option<String> {
    let schema = if let Some(r) = schema.get("$ref").and_then(|v| v.as_str()) {
        resolve_ref(root, r).unwrap_or(schema)
    } else {
        schema
    };

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
                    merged.insert(k.clone(), generate_property_example(v, root));
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
                obj.insert(key.clone(), generate_property_example(prop, root));
            }
        }
        return serde_json::to_string_pretty(&Value::Object(obj)).ok();
    }

    match schema_type? {
        "object" => {
            let mut obj = serde_json::Map::new();
            if let Some(props) = schema.get("properties").and_then(|v| v.as_object()) {
                for (key, prop) in props {
                    obj.insert(key.clone(), generate_property_example(prop, root));
                }
            }
            serde_json::to_string_pretty(&Value::Object(obj)).ok()
        }
        "array" => {
            if let Some(items) = schema.get("items") {
                let item = generate_property_example(items, root);
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

fn generate_property_example(schema: &Value, root: &Value) -> Value {
    let schema = if let Some(r) = schema.get("$ref").and_then(|v| v.as_str()) {
        resolve_ref(root, r).unwrap_or(schema)
    } else {
        schema
    };

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
                Value::Array(vec![generate_property_example(items, root)])
            } else {
                Value::Array(vec![])
            }
        }
        "object" => {
            let mut obj = serde_json::Map::new();
            if let Some(props) = schema.get("properties").and_then(|v| v.as_object()) {
                for (key, prop) in props {
                    obj.insert(key.clone(), generate_property_example(prop, root));
                }
            }
            Value::Object(obj)
        }
        _ => Value::Null,
    }
}
