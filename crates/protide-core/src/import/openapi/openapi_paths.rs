use std::collections::HashMap;

use serde_json::Value;

use super::ImportResult;
use super::openapi_operations::parse_operation;
use super::openapi_security::SecuritySchemeInfo;

#[allow(clippy::too_many_arguments)]
pub(super) fn parse_path_item(
    path: &str,
    item: &Value,
    base_url: &str,
    root: &Value,
    security_schemes: &HashMap<String, SecuritySchemeInfo>,
    global_consumes: &[String],
    global_security: &[String],
    result: &mut ImportResult,
) {
    let methods = ["get", "post", "put", "patch", "delete", "head", "options"];
    let path_params = item.get("parameters").and_then(|v| v.as_array()).cloned().unwrap_or_default();

    for method_str in methods {
        if let Some(operation) = item.get(method_str) {
            if let Some((folder, req)) = parse_operation(
                path,
                method_str,
                operation,
                base_url,
                root,
                &path_params,
                security_schemes,
                global_consumes,
                global_security,
            ) {
                result.add_request_in_folder(req, folder);
            }
        }
    }
}
