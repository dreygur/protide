use gpui::{Context, Window};
use super::*;

impl<E: WebSocketExecutor> RequestPanel<E> {
    /// Queue a deferred editor content update, applied on the next render.
    pub(super) fn queue_editor(&mut self, target: PendingEditor, content: String) {
        self.editor_pending.retain(|(t, _)| *t != target);
        self.editor_pending.push((target, content));
    }

    /// Apply all deferred editor content updates (called from render, which has Window).
    pub(super) fn apply_pending_editors(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.editor_pending.is_empty() { return; }
        for (target, content) in std::mem::take(&mut self.editor_pending) {
            let editor = match target {
                PendingEditor::Body => &self.body_editor,
                PendingEditor::PreScript => &self.pre_script_editor,
                PendingEditor::PostScript => &self.post_script_editor,
                PendingEditor::Tests => &self.tests_editor,
                PendingEditor::GraphqlQuery => &self.graphql_query_editor,
                PendingEditor::GraphqlVariables => &self.graphql_variables_editor,
                PendingEditor::GrpcMessage => &self.grpc_message_editor,
                PendingEditor::TrpcParams => &self.trpc_params_editor,
                PendingEditor::SioPayload => &self.sio_payload_editor,
                PendingEditor::TrpcPgResult => &self.trpc_pg_result_viewer,
                PendingEditor::TrpcPgAddInput => &self.trpc_pg_add_input,
            };
            editor.update(cx, |s, cx| s.set_value(&content, window, cx));
        }
    }

    /// Set body content in the editor
    pub fn set_body_content(&mut self, content: &str, _cx: &mut Context<Self>) {
        self.queue_editor(PendingEditor::Body, content.to_string());
        self.body = content.to_string();
    }

    /// Set variable extractions from @set annotations
    pub fn set_variable_extractions(&mut self, extractions: Vec<VariableExtraction>, cx: &mut Context<Self>) {
        self.variable_extractions = extractions;
        cx.notify();
    }

    pub(super) fn set_body_type(&mut self, body_type: BodyType, cx: &mut Context<Self>) {
        self.body_type = body_type;
        // Update editor highlighter language
        let lang = match body_type {
            BodyType::Json => "json",
            BodyType::Xml  => "xml",
            _              => "",
        };
        self.body_editor.update(cx, |s, cx| s.set_highlighter(lang, cx));
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
}
