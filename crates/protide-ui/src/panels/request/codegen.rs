use super::super::request_utils::{base64_encode, url_encode};
use super::*;
use gpui::{ClipboardItem, Context, Window};

impl<E: WebSocketExecutor> RequestPanel<E> {
    /// Generate code for current request using selected language
    pub fn generate_code(
        &mut self,
        language: CodegenLanguage,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut headers: Vec<(String, String)> = self
            .headers
            .iter()
            .filter(|h| h.enabled && !h.key.is_empty())
            .map(|h| (h.key.clone(), h.value.clone()))
            .collect();

        match self.auth_type {
            AuthType::Bearer if !self.bearer_token.is_empty() => {
                headers.push((
                    "Authorization".to_string(),
                    format!("Bearer {}", self.bearer_token),
                ));
            }
            AuthType::Basic
                if !self.basic_username.is_empty() || !self.basic_password.is_empty() =>
            {
                let credentials = format!("{}:{}", self.basic_username, self.basic_password);
                let encoded = base64_encode(credentials.as_bytes());
                headers.push(("Authorization".to_string(), format!("Basic {}", encoded)));
            }
            AuthType::ApiKey
                if !self.api_key_name.is_empty()
                    && self.api_key_location == ApiKeyLocation::Header =>
            {
                headers.push((self.api_key_name.clone(), self.api_key_value.clone()));
            }
            _ => {}
        }

        let body = match self.body_type {
            BodyType::Form => {
                let has_files = self.form_data.iter()
                    .any(|f| f.enabled && f.field_type == FormFieldType::File && f.file_path.is_some());
                if has_files {
                    // File uploads aren't representable as a single body
                    // string across curl/Python/JS/Go/Rust without a
                    // per-language multipart implementation; say so plainly
                    // rather than silently emitting an empty body.
                    Some("# File upload bodies aren't supported by code generation yet - attach the file manually.".to_string())
                } else {
                    let s = self.form_data.iter()
                        .filter(|f| f.enabled && !f.key.is_empty())
                        .map(|f| format!("{}={}", url_encode(&f.key), url_encode(&f.value)))
                        .collect::<Vec<_>>().join("&");
                    if s.is_empty() { None } else { Some(s) }
                }
            }
            BodyType::Binary => self.binary_file_path.as_ref().map(|p| {
                format!("# Binary body from file: {} - read and send this file's bytes as the request body.", p.display())
            }),
            _ => {
                let body = self.body_editor.read(cx).value().to_string();
                if body.trim().is_empty() { None } else { Some(body) }
            }
        };

        let request = protide_core::codegen::CodegenRequest {
            method: self.method.as_str().to_string(),
            url: self.url.clone(),
            headers,
            body,
        };

        let code = protide_core::codegen::generate(&request, language);
        self.codegen_language = language;
        self.codegen_content = Some(code.clone());
        let editor_lang = match language {
            CodegenLanguage::Curl => "sh",
            CodegenLanguage::Python => "python",
            CodegenLanguage::JavaScript => "javascript",
            CodegenLanguage::Go => "go",
            CodegenLanguage::Rust => "rust",
        };
        self.codegen_editor.update(cx, |s, cx| {
            s.set_value(&code, window, cx);
            s.set_highlighter(editor_lang, cx);
        });
        cx.notify();
    }

    pub fn codegen_lang_name(&self) -> &'static str {
        match self.codegen_language {
            CodegenLanguage::Curl => "cURL",
            CodegenLanguage::Python => "Python",
            CodegenLanguage::JavaScript => "JavaScript",
            CodegenLanguage::Go => "Go",
            CodegenLanguage::Rust => "Rust",
        }
    }

    pub fn close_codegen_panel(&mut self, cx: &mut Context<Self>) {
        self.codegen_content = None;
        cx.notify();
    }

    pub fn copy_generated_code(&self, cx: &mut Context<Self>) {
        if let Some(code) = &self.codegen_content {
            cx.write_to_clipboard(ClipboardItem::new_string(code.clone()));
        }
    }
}
