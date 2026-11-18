use super::*;
use gpui::{ClipboardItem, Context, Window};

// File uploads aren't representable as a single body string across
// curl/Python/JS/Go/Rust without a per-language multipart implementation;
// say so plainly rather than silently emitting an empty body.
const FILE_UPLOAD_NOTE: &str =
    "# File upload bodies aren't supported by code generation yet; attach the file manually.";

impl<E: WebSocketExecutor> RequestPanel<E> {
    /// Generate code for current request using selected language
    pub fn generate_code(
        &mut self,
        language: CodegenLanguage,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let env_state = self
            .explorer_panel
            .as_ref()
            .map(|panel| panel.read(cx).env_state().clone());
        let substitute = |s: &str| {
            env_state
                .as_ref()
                .map_or_else(|| s.to_string(), |env| env.substitute(s))
        };

        let mut headers: Vec<(String, String)> = self
            .headers
            .iter()
            .filter(|h| h.enabled && !h.key.is_empty())
            .map(|h| (substitute(&h.key), substitute(&h.value)))
            .collect();
        headers.extend(self.build_auth_headers(&substitute));

        let body = match self.body_type {
            BodyType::Binary => self.binary_file_path.as_ref().map(|p| {
                format!("# Binary body from file: {}; read and send this file's bytes as the request body.", p.display())
            }),
            _ => {
                let body_content = self.body_editor.read(cx).value().to_string();
                match self.build_execution_body(false, &self.method, &body_content, &substitute) {
                    ExecutionBody::Multipart(_) => Some(FILE_UPLOAD_NOTE.to_string()),
                    exec_body => exec_body.as_text(),
                }
            }
        };

        let request = protide_core::codegen::CodegenRequest {
            method: self.method.as_str().to_string(),
            url: self.with_auth_query(substitute(&self.url), &substitute),
            headers,
            body,
            content_type: self.body_type.content_type().map(str::to_string),
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
