use super::super::request_utils::{base64_encode, split_grpc_url, url_encode};
use super::init::EMPTY_JSON;
use super::*;
use gpui::Context;
use http_parser::Protocol;

fn mode_protocol(mode: RequestMode) -> Protocol {
    match mode {
        RequestMode::Http => Protocol::Http,
        RequestMode::GraphQL => Protocol::GraphQL,
        RequestMode::WebSocket => Protocol::WebSocket,
        RequestMode::Grpc => Protocol::Grpc,
        RequestMode::Trpc => Protocol::Trpc,
        RequestMode::SocketIo => Protocol::SocketIO,
    }
}

fn non_empty(text: String) -> Option<String> {
    (!text.trim().is_empty()).then_some(text)
}

impl<E: WebSocketExecutor> RequestPanel<E> {
    /// Save the current request to a .http file. Returns whether it was
    /// written in place; a request without a file opens a save dialog.
    pub fn save_request(&mut self, cx: &mut Context<Self>) -> bool {
        // A reopened file must not ask about scripts the user wrote here.
        self.trust_edited_scripts(cx);
        let request = self.build_request(cx);

        // Save in place if a file is already loaded: only the loaded request's
        // block is rewritten, every other request in the file is kept as is.
        if let Some(ref path) = self.current_file.clone() {
            let index = self.current_request_index;
            let content = std::fs::read_to_string(path)
                .ok()
                .and_then(|old| http_parser::replace_request(&old, index, &request));
            let Some(content) = content else {
                log::error!(
                    "Not saved: request {} of {} could not be found",
                    index + 1,
                    path.display()
                );
                return false;
            };
            if let Err(e) = std::fs::write(path, &content) {
                log::error!("Failed to save request {}: {}", path.display(), e);
                return false;
            }
            if self.external_change_pending {
                log::warn!("Overwrote external changes to {}", path.display());
            }
            log::info!("Saved: {}", path.display());
            self.set_loaded_file(path.clone(), index, request, content, cx);
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
            return true;
        }

        // Otherwise open save dialog
        let default_name = if self.url.is_empty() {
            "new-request.http".to_string()
        } else {
            let name = self
                .url
                .split('/')
                .rfind(|s| !s.is_empty() && !s.contains("://") && !s.contains('.'))
                .unwrap_or("request");
            format!("{}.http", name)
        };
        let title = if self.url.is_empty() {
            "New Request"
        } else {
            &self.url
        };
        let content = format!(
            "### {}\n\n{}",
            http_parser::single_line(title),
            request.to_http()
        );

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
            move |this, path, cx| {
                last_paths::save_last_dir("save_request", &path);
                let path = if path.extension().is_none_or(|ext| ext != "http") {
                    path.with_extension("http")
                } else {
                    path
                };
                if let Err(e) = std::fs::write(&path, &content) {
                    log::error!("Failed to save request {}: {}", path.display(), e);
                } else {
                    log::info!("Saved: {}", path.display());
                    this.set_loaded_file(path, 0, request, content, cx);
                }
            },
        );
        false
    }

    /// The editor state as a parsed request. Starts from the request loaded
    /// from the file, so annotations the editor does not show (`@name`,
    /// `@description`, `@depends`, `@set`) are kept as they were.
    pub(super) fn build_request(&self, cx: &Context<Self>) -> http_parser::Request {
        let mut req = self.loaded_request.clone().unwrap_or_else(|| {
            http_parser::Request::new(http_parser::HttpMethod::Get, String::new())
        });
        // Only HTTP and tRPC show a method; GraphQL always posts, and the
        // other protocols keep the method word the file used (e.g. GRPC).
        let method = match self.request_mode {
            RequestMode::GraphQL => Some("POST"),
            RequestMode::Http | RequestMode::Trpc => Some(self.method.as_str()),
            _ => None,
        };
        if let Some(m) = method.and_then(http_parser::HttpMethod::from_str) {
            req.method = m;
        }
        req.url = match self.request_mode {
            RequestMode::Trpc => format!("{}{}", self.url, self.trpc_procedure),
            // The picked method, else the one the file named.
            RequestMode::Grpc => {
                let path = self.grpc_method.as_ref().map(|m| m.full_name.as_str());
                match path.or_else(|| split_grpc_url(&req.url).1) {
                    Some(path) => format!("{}/{}", self.url.trim_end_matches('/'), path),
                    None => self.url.clone(),
                }
            }
            _ => self.url.clone(),
        };

        // Only write @protocol when the method alone would not imply it, or
        // when the file already spelled it out.
        let protocol = mode_protocol(self.request_mode);
        let had_protocol = req.meta.protocol.is_some();
        req.meta.protocol = None;
        if had_protocol || req.protocol() != protocol {
            req.meta.protocol = Some(protocol);
        }
        if self.request_mode == RequestMode::Grpc
            && let Some(ref proto_path) = self.grpc_proto_path
        {
            req.meta.proto_path = Some(proto_path.display().to_string());
        }

        req.headers = self
            .headers
            .iter()
            .filter(|h| !h.key.is_empty())
            .map(|h| http_parser::KeyValue {
                key: h.key.clone(),
                value: h.value.clone(),
                enabled: h.enabled,
            })
            .collect();
        if let Some((key, value)) = self.auth_header() {
            req.headers.push(http_parser::KeyValue::new(key, value));
        }

        // A load fills the gRPC and tRPC editors with "{}" when the file has
        // no body; that untouched default is not written back as one.
        let body = non_empty(self.generate_body_content(cx));
        let default_only = matches!(self.request_mode, RequestMode::Grpc | RequestMode::Trpc)
            && req.body.is_none()
            && body.as_deref() == Some(EMPTY_JSON);
        req.body = body.filter(|_| !default_only);
        req.scripts = http_parser::Scripts {
            pre_script: non_empty(self.pre_script_editor.read(cx).value().to_string()),
            post_script: non_empty(self.post_script_editor.read(cx).value().to_string()),
            tests: non_empty(self.tests_editor.read(cx).value().to_string()),
        };
        req
    }

    /// The auth tab as the header it is saved as, if it saves to one.
    fn auth_header(&self) -> Option<(String, String)> {
        match self.auth_type {
            AuthType::None => None,
            AuthType::Bearer => (!self.bearer_token.is_empty()).then(|| {
                (
                    "Authorization".to_string(),
                    format!("Bearer {}", self.bearer_token),
                )
            }),
            AuthType::Basic => (!self.basic_username.is_empty() || !self.basic_password.is_empty())
                .then(|| {
                    let credentials = format!("{}:{}", self.basic_username, self.basic_password);
                    (
                        "Authorization".to_string(),
                        format!("Basic {}", base64_encode(credentials.as_bytes())),
                    )
                }),
            AuthType::ApiKey => (!self.api_key_name.is_empty()
                && !self.api_key_value.is_empty()
                && self.api_key_location == ApiKeyLocation::Header)
                .then(|| (self.api_key_name.clone(), self.api_key_value.clone())),
        }
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
    /// gRPC and tRPC bodies come from their own editors, which is where a
    /// load puts them. Socket.IO payloads are not loaded from the body, so
    /// that mode still passes the plain body editor through.
    fn generate_body_content(&self, cx: &Context<Self>) -> String {
        if self.request_mode == RequestMode::GraphQL {
            let query = self.graphql_query_editor.read(cx).value().to_string();
            let variables = self.graphql_variables_editor.read(cx).value().to_string();
            let mut obj = serde_json::Map::new();
            obj.insert("query".to_string(), serde_json::Value::String(query));
            if !variables.trim().is_empty()
                && let Ok(v) = serde_json::from_str::<serde_json::Value>(&variables)
            {
                obj.insert("variables".to_string(), v);
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

        let editor = match self.request_mode {
            RequestMode::Grpc => &self.grpc_message_editor,
            RequestMode::Trpc => &self.trpc_params_editor,
            _ => &self.body_editor,
        };
        editor.read(cx).value().to_string()
    }
}
