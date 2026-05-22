use gpui::{Context, Window};
use super::*;

impl<E: WebSocketExecutor> RequestPanel<E> {
    pub fn open_import_modal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.import_modal_open = true;
        self.import_text.clear();
        self.import_error = None;
        self.import_editor.update(cx, |s, cx| s.set_value("", window, cx));
        cx.notify();
    }

    pub fn close_import_modal(&mut self, cx: &mut Context<Self>) {
        self.import_modal_open = false;
        self.import_text.clear();
        self.import_error = None;
        cx.notify();
    }

    pub(super) fn set_import_text(&mut self, text: String, window: &mut Window, cx: &mut Context<Self>) {
        self.import_editor.update(cx, |s, cx| s.set_value(&text, window, cx));
        self.import_text = text;
        self.import_error = None;
        cx.notify();
    }

    pub(super) fn browse_import_file(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let start_dir = last_paths::last_dir("import_collection").or_else(dirs::home_dir);
        let mut dialog = rfd::FileDialog::new()
            .set_title("Import Collection")
            .add_filter("All Supported", &["json", "yaml", "yml", "bru", "txt", "curl"])
            .add_filter("Postman Collection", &["json"])
            .add_filter("OpenAPI/Swagger", &["json", "yaml", "yml"])
            .add_filter("Bruno Collection", &["bru"])
            .add_filter("cURL Command", &["txt", "curl"])
            .add_filter("All Files", &["*"]);

        if let Some(dir) = start_dir {
            dialog = dialog.set_directory(dir);
        }

        if let Some(path) = dialog.pick_file() {
            last_paths::save_last_dir("import_collection", &path);
            match std::fs::read_to_string(&path) {
                Ok(content) => { self.set_import_text(content, window, cx); }
                Err(e) => { self.import_error = Some(format!("Failed to read file: {}", e)); }
            }
        }
        cx.notify();
    }

    pub(super) fn execute_import(&mut self, cx: &mut Context<Self>) {
        let editor_content = self.import_editor.read(cx).value().to_string();
        if !editor_content.is_empty() {
            self.import_text = editor_content;
        }
        if self.import_text.trim().is_empty() {
            self.import_error = Some("Please paste a cURL command or request data".to_string());
            cx.notify();
            return;
        }

        match protide_core::import::import(&self.import_text) {
            Ok(result) => {
                if let Some(request) = result.requests.into_iter().next() {
                    let method = match request.method {
                        http_parser::HttpMethod::Get => HttpMethod::Get,
                        http_parser::HttpMethod::Post => HttpMethod::Post,
                        http_parser::HttpMethod::Put => HttpMethod::Put,
                        http_parser::HttpMethod::Patch => HttpMethod::Patch,
                        http_parser::HttpMethod::Delete => HttpMethod::Delete,
                        _ => HttpMethod::Get,
                    };

                    self.method = method;
                    self.url = request.url;
                    let len = self.url.chars().count();
                    self.url_selection = len..len;

                    self.headers = request.headers
                        .into_iter()
                        .map(|h| KeyValuePair { key: h.key, value: h.value, enabled: h.enabled })
                        .collect();
                    self.headers.push(KeyValuePair::default());

                    if let Some(body) = request.body {
                        self.body = body.clone();
                        self.queue_editor(PendingEditor::Body, body);
                        let is_json = self.headers.iter().any(|h| {
                            h.key.eq_ignore_ascii_case("content-type") && h.value.contains("json")
                        });
                        self.body_type = if is_json { BodyType::Json } else { BodyType::Raw };
                    }

                    self.import_modal_open = false;
                    self.import_text.clear();
                    self.import_error = None;
                } else {
                    self.import_error = Some("No request found in import data".to_string());
                }
            }
            Err(e) => { self.import_error = Some(e); }
        }
        cx.notify();
    }
}
