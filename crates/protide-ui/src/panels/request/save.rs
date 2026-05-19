use gpui::Context;
use super::*;
use super::super::request_utils::{base64_encode, url_encode};

impl<E: WebSocketExecutor> RequestPanel<E> {
    /// Save the current request to a .http file
    pub fn save_request(&mut self, cx: &mut Context<Self>) {
        let content = self.generate_http_content(cx);

        // Save in-place if a file is already loaded
        if let Some(ref path) = self.current_file.clone() {
            if let Err(e) = std::fs::write(path, &content) {
                log::error!("Failed to save request {}: {}", path.display(), e);
            } else {
                log::info!("Saved: {}", path.display());
                self.save_feedback = true;
                cx.notify();
                cx.spawn(async move |this, cx| {
                    cx.background_executor().timer(std::time::Duration::from_millis(1500)).await;
                    this.update(cx, |this, cx| {
                        this.save_feedback = false;
                        cx.notify();
                    }).ok();
                }).detach();
            }
            return;
        }

        // Otherwise open save dialog
        let default_name = if self.url.is_empty() {
            "new-request.http".to_string()
        } else {
            let name = self.url.split('/')
                .filter(|s| !s.is_empty() && !s.contains("://") && !s.contains('.'))
                .last()
                .unwrap_or("request");
            format!("{}.http", name)
        };

        let start_dir = last_paths::last_dir("save_request").or_else(dirs::home_dir);
        let mut dialog = rfd::FileDialog::new()
            .set_title("Save Request")
            .set_file_name(&default_name)
            .add_filter("HTTP Request", &["http"]);
        if let Some(dir) = start_dir {
            dialog = dialog.set_directory(dir);
        }

        if let Some(path) = dialog.save_file() {
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
                self.current_file = Some(path);
            }
        }
    }

    /// Generate .http file content from current request state
    pub(super) fn generate_http_content(&self, cx: &Context<Self>) -> String {
        let mut lines = Vec::new();

        let name = if self.url.is_empty() { "New Request" } else { &self.url };
        lines.push(format!("### {}", name));
        lines.push(String::new());

        if let Some(ref proto_path) = self.grpc_proto_path {
            lines.push(format!("# @proto {}", proto_path.display()));
        }

        lines.push(format!("{} {}", self.method.as_str(), self.url));

        for header in &self.headers {
            if header.enabled && !header.key.is_empty() {
                lines.push(format!("{}: {}", header.key, header.value));
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

        let body_content = self.body_editor.read(cx).content().to_string();
        if !body_content.is_empty() {
            lines.push(String::new());
            lines.push(body_content);
        }

        lines.join("\n")
    }
}

// Suppress unused import warning — url_encode is used transitively via url_sync
#[allow(unused_imports)]
use super::super::request_utils::url_encode as _url_encode_unused;
