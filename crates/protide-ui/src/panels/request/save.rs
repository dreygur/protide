use super::super::request_utils::{base64_encode, url_encode};
use super::*;
use gpui::Context;

impl<E: WebSocketExecutor> RequestPanel<E> {
    /// Save the current request to a .http file
    pub fn save_request(&mut self, cx: &mut Context<Self>) {
        let content = self.generate_http_content(cx);

        // Save in-place if a file is already loaded
        if let Some(ref path) = self.current_file.clone() {
            if let Err(e) = std::fs::write(path, &content) {
                log::error!("Failed to save request {}: {}", path.display(), e);
            } else {
                if self.external_change_pending {
                    log::warn!("Overwrote external changes to {}", path.display());
                }
                log::info!("Saved: {}", path.display());
                self.external_change_pending = false;
                self.save_feedback = true;
                cx.notify();
                cx.spawn(async move |this, cx| {
                    cx.background_executor()
                        .timer(std::time::Duration::from_millis(1500))
                        .await;
                    this.update(cx, |this, cx| {
                        this.save_feedback = false;
                        cx.notify();
                    })
                    .ok();
                })
                .detach();
            }
            return;
        }

        // Otherwise open save dialog
        let default_name = if self.url.is_empty() {
            "new-request.http".to_string()
        } else {
            let name = self
                .url
                .split('/')
                .filter(|s| !s.is_empty() && !s.contains("://") && !s.contains('.'))
                .last()
                .unwrap_or("request");
            format!("{}.http", name)
        };

        let start_dir = last_paths::last_dir("save_request").or_else(dirs::home_dir);
        file_dialog::prompt(
            cx,
            Pick::Save,
            move |d| {
                let d = d
                    .set_title("Save Request")
                    .set_file_name(&default_name)
                    .add_filter("HTTP Request", &["http"]);
                match start_dir {
                    Some(dir) => d.set_directory(dir),
                    None => d,
                }
            },
            move |this, path, _cx| {
                last_paths::save_last_dir("save_request", &path);
                let path = if path.extension().map_or(true, |ext| ext != "http") {
                    path.with_extension("http")
                } else {
                    path
                };
                if let Err(e) = std::fs::write(&path, &content) {
                    log::error!("Failed to save request {}: {}", path.display(), e);
                } else {
                    log::info!("Saved: {}", path.display());
                    this.current_file = Some(path);
                }
            },
        );
    }

    /// Generate .http file content from current request state
    pub(super) fn generate_http_content(&self, cx: &Context<Self>) -> String {
        let mut lines = Vec::new();

        let name = if self.url.is_empty() {
            "New Request"
        } else {
            &self.url
        };
        lines.push(format!("### {}", name));
        lines.push(String::new());

        if self.request_mode == RequestMode::GraphQL {
            lines.push("# @protocol graphql".to_string());
        }

        if let Some(ref proto_path) = self.grpc_proto_path {
            lines.push(format!("# @proto {}", proto_path.display()));
        }

        let method = if self.request_mode == RequestMode::GraphQL {
            "POST"
        } else {
            self.method.as_str()
        };
        lines.push(format!("{} {}", method, self.url));

        // Disabled headers are written as `# Key: Value` comment lines so the
        // http-parser can round-trip the enabled/disabled state on load.
        for header in &self.headers {
            if header.key.is_empty() {
                continue;
            }
            if header.enabled {
                lines.push(format!("{}: {}", header.key, header.value));
            } else {
                lines.push(format!("# {}: {}", header.key, header.value));
            }
        }

        match self.auth_type {
            AuthType::None => {}
            AuthType::Bearer => {
                if !self.bearer_token.is_empty() {
                    lines.push(format!("Authorization: Bearer {}", self.bearer_token));
                }
            }
            AuthType::Basic => {
                if !self.basic_username.is_empty() || !self.basic_password.is_empty() {
                    let credentials = format!("{}:{}", self.basic_username, self.basic_password);
                    let encoded = base64_encode(credentials.as_bytes());
                    lines.push(format!("Authorization: Basic {}", encoded));
                }
            }
            AuthType::ApiKey => {
                if !self.api_key_name.is_empty() && !self.api_key_value.is_empty() {
                    if self.api_key_location == ApiKeyLocation::Header {
                        lines.push(format!("{}: {}", self.api_key_name, self.api_key_value));
                    }
                }
            }
        }

        let body_content = self.generate_body_content(cx);
        if !body_content.is_empty() {
            lines.push(String::new());
            lines.push(body_content);
        }

        lines.join("\n")
    }

    /// Build the saved body text for the current mode/body type. GraphQL is
    /// serialized as the same `{"query", "variables", "operationName"}` JSON
    /// object the executor sends (see `execution_http.rs`) and that
    /// `draft_load.rs` already knows how to read back. Form bodies without
    /// file attachments are serialized the same way they're sent on the wire
    /// (url-encoded text), which is plain body text the parser already
    /// supports. Binary bodies are read from disk and written as text
    /// (lossy for genuinely non-UTF8 files - see limitation note below).
    ///
    /// gRPC/tRPC/Socket.IO bodies are left as-is (body_editor passthrough,
    /// matching prior behavior): those protocols have their own dedicated
    /// editors (`grpc_message_editor`, `trpc_params_editor`,
    /// `sio_payload_editor`) that this function does not read, so saving
    /// those modes is a known follow-up rather than something this fix
    /// covers.
    fn generate_body_content(&self, cx: &Context<Self>) -> String {
        if self.request_mode == RequestMode::GraphQL {
            let query = self.graphql_query_editor.read(cx).value().to_string();
            let variables = self.graphql_variables_editor.read(cx).value().to_string();
            let mut obj = serde_json::Map::new();
            obj.insert("query".to_string(), serde_json::Value::String(query));
            if !variables.trim().is_empty() {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&variables) {
                    obj.insert("variables".to_string(), v);
                }
            }
            if !self.graphql_operation_name.trim().is_empty() {
                obj.insert(
                    "operationName".to_string(),
                    serde_json::Value::String(self.graphql_operation_name.clone()),
                );
            }
            return serde_json::to_string_pretty(&serde_json::Value::Object(obj))
                .unwrap_or_default();
        }

        if self.request_mode == RequestMode::Http && self.body_type == BodyType::Form {
            let has_files = self
                .form_data
                .iter()
                .any(|f| f.enabled && f.field_type == FormFieldType::File && f.file_path.is_some());
            let encoded_fields = self
                .form_data
                .iter()
                .filter(|f| f.enabled && !f.key.is_empty() && f.field_type != FormFieldType::File)
                .map(|f| format!("{}={}", url_encode(&f.key), url_encode(&f.value)))
                .collect::<Vec<_>>()
                .join("&");
            if has_files {
                // File attachments can't be represented as text in a .http
                // file; only the text fields are preserved.
                let note = "# NOTE: file attachments are not preserved when saving to .http; re-attach them after reopening this request.";
                return if encoded_fields.is_empty() {
                    note.to_string()
                } else {
                    format!("{}\n{}", encoded_fields, note)
                };
            }
            return encoded_fields;
        }

        if self.request_mode == RequestMode::Http && self.body_type == BodyType::Binary {
            return self
                .binary_file_path
                .as_ref()
                .and_then(|p| std::fs::read(p).ok())
                .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
                .unwrap_or_default();
        }

        self.body_editor.read(cx).value().to_string()
    }
}
