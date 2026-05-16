use gpui::Context;
use super::*;
use crate::ui::components::code_editor::Language;

impl<E: WebSocketExecutor> RequestPanel<E> {
    /// Set body content in the CodeEditor
    pub fn set_body_content(&mut self, content: &str, cx: &mut Context<Self>) {
        self.body_editor.update(cx, |editor, cx| {
            editor.set_content(content, cx);
        });
        self.body = content.to_string();
    }

    /// Set variable extractions from @set annotations
    pub fn set_variable_extractions(&mut self, extractions: Vec<VariableExtraction>, cx: &mut Context<Self>) {
        self.variable_extractions = extractions;
        cx.notify();
    }

    pub(super) fn set_body_type(&mut self, body_type: BodyType, cx: &mut Context<Self>) {
        self.body_type = body_type;
        // Update CodeEditor language
        let lang = match body_type {
            BodyType::Json => Language::Json,
            BodyType::Xml  => Language::Xml,
            _              => Language::Plain,
        };
        self.body_editor.update(cx, |ed, cx| ed.set_language(lang, cx));
        // Update Content-Type header
        let content_type = match body_type {
            BodyType::Json   => "application/json",
            BodyType::Xml    => "application/xml",
            BodyType::Form   => "application/x-www-form-urlencoded",
            BodyType::Raw    => "text/plain",
            BodyType::Binary => return cx.notify(), // no content-type update for binary
        };
        if let Some(header) = self.headers.iter_mut().find(|h| h.key.eq_ignore_ascii_case("content-type")) {
            header.value = content_type.to_string();
        } else {
            self.headers.insert(0, KeyValuePair {
                key: "Content-Type".to_string(),
                value: content_type.to_string(),
                enabled: true,
            });
        }
        cx.notify();
    }

    pub(super) fn browse_binary_file(&mut self, cx: &mut Context<Self>) {
        let mut dialog = rfd::FileDialog::new().set_title("Select Binary File");
        if let Some(dir) = last_paths::last_dir("binary_file").or_else(dirs::home_dir) {
            dialog = dialog.set_directory(dir);
        }
        if let Some(path) = dialog.pick_file() {
            last_paths::save_last_dir("binary_file", &path);
            self.binary_file_path = Some(path);
            cx.notify();
        }
    }

    pub(super) fn browse_client_cert(&mut self, cx: &mut Context<Self>) {
        let mut dialog = rfd::FileDialog::new()
            .set_title("Select Client Certificate (PEM)")
            .add_filter("PEM Certificate", &["pem", "crt", "cer"]);
        if let Some(dir) = last_paths::last_dir("client_cert").or_else(dirs::home_dir) {
            dialog = dialog.set_directory(dir);
        }
        if let Some(path) = dialog.pick_file() {
            last_paths::save_last_dir("client_cert", &path);
            self.client_cert_path = Some(path);
            cx.notify();
        }
    }

    pub(super) fn browse_client_key(&mut self, cx: &mut Context<Self>) {
        let mut dialog = rfd::FileDialog::new()
            .set_title("Select Private Key (PEM)")
            .add_filter("PEM Key", &["pem", "key"]);
        if let Some(dir) = last_paths::last_dir("client_key").or_else(dirs::home_dir) {
            dialog = dialog.set_directory(dir);
        }
        if let Some(path) = dialog.pick_file() {
            last_paths::save_last_dir("client_key", &path);
            self.client_key_path = Some(path);
            cx.notify();
        }
    }
}
