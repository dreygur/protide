use super::super::request_utils::url_encode;
use super::*;

impl<E: WebSocketExecutor> RequestPanel<E> {
    /// Add the body type's Content-Type for a text body unless the user set
    /// one. Multipart is left alone: reqwest adds it with the boundary.
    pub(super) fn add_body_content_type(
        &self,
        headers: &mut Vec<(String, String)>,
        body: &ExecutionBody,
    ) {
        if let (ExecutionBody::Text(_), Some(content_type)) = (body, self.body_type.content_type())
        {
            add_default_header(headers, "Content-Type", content_type);
        }
    }

    pub(super) fn build_execution_body<F>(
        &self,
        is_graphql_mode: bool,
        method: &HttpMethod,
        body_content: &str,
        substitute: &F,
    ) -> ExecutionBody
    where
        F: Fn(&str) -> String,
    {
        let binary_file_path = self.binary_file_path.clone();
        let has_files = self.body_type == BodyType::Form
            && self
                .form_data
                .iter()
                .any(|f| f.enabled && f.field_type == FormFieldType::File && f.file_path.is_some());

        let form_fields: Vec<(String, String, Option<std::path::PathBuf>, bool)> =
            if self.body_type == BodyType::Form {
                self.form_data
                    .iter()
                    .filter(|f| f.enabled && !f.key.is_empty())
                    .map(|f| {
                        (
                            substitute(&f.key),
                            substitute(&f.value),
                            f.file_path.clone(),
                            f.field_type == FormFieldType::File,
                        )
                    })
                    .collect()
            } else {
                Vec::new()
            };

        if is_graphql_mode {
            ExecutionBody::None
        } else if matches!(
            method,
            HttpMethod::Post | HttpMethod::Put | HttpMethod::Patch | HttpMethod::Custom(_)
        ) {
            match self.body_type {
                BodyType::Form if !has_files => {
                    let s = form_fields
                        .iter()
                        .filter(|(_, _, _, is_file)| !is_file)
                        .map(|(k, v, _, _)| format!("{}={}", url_encode(k), url_encode(v)))
                        .collect::<Vec<_>>()
                        .join("&");
                    if s.is_empty() {
                        ExecutionBody::None
                    } else {
                        ExecutionBody::Text(s)
                    }
                }
                BodyType::Form => ExecutionBody::Multipart(
                    form_fields
                        .iter()
                        .map(|(k, v, path, is_file)| FormPart {
                            name: k.clone(),
                            value: if *is_file {
                                FormPartValue::File(path.clone().unwrap_or_default())
                            } else {
                                FormPartValue::Text(v.clone())
                            },
                        })
                        .collect(),
                ),
                BodyType::Binary => binary_file_path
                    .as_ref()
                    .and_then(|p| std::fs::read(p).ok())
                    .map(ExecutionBody::Binary)
                    .unwrap_or(ExecutionBody::None),
                _ => ExecutionBody::Text(substitute(body_content)),
            }
        } else {
            ExecutionBody::None
        }
    }
}
