use gpui::{ClipboardItem, Context};
use super::*;
use super::super::request_utils::base64_encode;
use crate::components::code_editor::Language;

impl<E: WebSocketExecutor> RequestPanel<E> {
    /// Generate code for current request using selected language
    pub fn generate_code(&mut self, language: CodegenLanguage, cx: &mut Context<Self>) {
        let mut headers: Vec<(String, String)> = self
            .headers
            .iter()
            .filter(|h| h.enabled && !h.key.is_empty())
            .map(|h| (h.key.clone(), h.value.clone()))
            .collect();

        match self.auth_type {
            AuthType::Bearer if !self.bearer_token.is_empty() => {
                headers.push(("Authorization".to_string(), format!("Bearer {}", self.bearer_token)));
            }
            AuthType::Basic if !self.basic_username.is_empty() || !self.basic_password.is_empty() => {
                let credentials = format!("{}:{}", self.basic_username, self.basic_password);
                let encoded = base64_encode(credentials.as_bytes());
                headers.push(("Authorization".to_string(), format!("Basic {}", encoded)));
            }
            AuthType::ApiKey if !self.api_key_name.is_empty() && self.api_key_location == ApiKeyLocation::Header => {
                headers.push((self.api_key_name.clone(), self.api_key_value.clone()));
            }
            _ => {}
        }

        let body = self.body_editor.read(cx).content().to_string();
        let body = if body.trim().is_empty() { None } else { Some(body) };

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
            CodegenLanguage::Curl       => Language::Shell,
            CodegenLanguage::Python     => Language::Python,
            CodegenLanguage::JavaScript => Language::JavaScript,
            CodegenLanguage::Go         => Language::Go,
            CodegenLanguage::Rust       => Language::Rust,
        };
        self.codegen_editor.update(cx, |editor, cx| {
            editor.set_content(&code, cx);
            editor.set_language(editor_lang, cx);
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
