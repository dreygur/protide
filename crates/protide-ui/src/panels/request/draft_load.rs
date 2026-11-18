use super::super::request_utils::split_grpc_url;
use super::init::{DEFAULT_GRAPHQL_QUERY, EMPTY_JSON};
use super::*;

impl<E: WebSocketExecutor> RequestPanel<E> {
    /// Reset every per request field to its default so a loaded request that
    /// leaves a field undefined never inherits the previous request's value.
    /// Auth is reset too: a saved request carries it as a plain header.
    fn reset_request_fields(&mut self, cx: &mut Context<Self>) {
        self.set_body_content("", cx);
        self.body_type = BodyType::Json;
        self.form_data = vec![FormField::default()];
        self.binary_file_path = None;
        self.auth_type = AuthType::None;
        self.bearer_token.clear();
        self.basic_username.clear();
        self.basic_password.clear();
        self.api_key_name.clear();
        self.api_key_value.clear();
        self.api_key_location = ApiKeyLocation::Header;
        self.queue_editor(
            PendingEditor::GraphqlQuery,
            DEFAULT_GRAPHQL_QUERY.to_string(),
        );
        self.queue_editor(PendingEditor::GraphqlVariables, EMPTY_JSON.to_string());
        self.graphql_operation_name.clear();
        self.queue_editor(PendingEditor::GrpcMessage, EMPTY_JSON.to_string());
        self.queue_editor(PendingEditor::TrpcParams, EMPTY_JSON.to_string());
        self.trpc_procedure.clear();
    }

    /// Load request data from a history entry
    pub fn load_from_history(
        &mut self,
        method: String,
        url: String,
        headers: Vec<(String, String)>,
        body: Option<String>,
        cx: &mut Context<Self>,
    ) {
        self.reset_request_fields(cx);
        self.clear_loaded_file(cx);
        if let Some(m) = HttpMethod::from_str(&method) {
            self.method = m;
        }

        self.url = url;
        let char_count = self.url.chars().count();
        self.url_selection = char_count..char_count;

        self.headers = headers
            .into_iter()
            .map(|(key, value)| KeyValuePair {
                key,
                value,
                enabled: true,
            })
            .collect();
        self.headers.push(KeyValuePair::default());

        if let Some(b) = body {
            self.set_body_content(&b, cx);
        }

        self.sync_params_from_url(cx);
        self.active_edit = None;
        self.method_dropdown_open = false;
        self.variable_extractions.clear();

        cx.notify();
    }

    /// Load a parsed request from a .http file, switching protocol as needed.
    pub fn load_from_parsed_request(&mut self, req: &http_parser::Request, cx: &mut Context<Self>) {
        use http_parser::Protocol;

        self.reset_request_fields(cx);
        self.request_mode = match req.protocol() {
            Protocol::Http => RequestMode::Http,
            Protocol::GraphQL => RequestMode::GraphQL,
            Protocol::WebSocket => RequestMode::WebSocket,
            Protocol::Grpc => RequestMode::Grpc,
            Protocol::SocketIO => RequestMode::SocketIo,
            Protocol::Trpc => RequestMode::Trpc,
        };
        self.active_tab = 0;
        self.active_edit = None;
        self.method_dropdown_open = false;
        self.variable_extractions.clear();

        // Disabled headers load as disabled rows so a save writes them back.
        self.headers = req
            .headers
            .iter()
            .map(|h| KeyValuePair {
                key: h.key.clone(),
                value: h.value.clone(),
                enabled: h.enabled,
            })
            .collect();
        self.headers.push(KeyValuePair::default());

        match req.protocol() {
            Protocol::Http => {
                if let Some(m) = HttpMethod::from_str(req.method.as_str()) {
                    self.method = m;
                }
                self.url = req.url.clone();
                let len = self.url.chars().count();
                self.url_selection = len..len;
                if let Some(body) = &req.body {
                    self.set_body_content(body, cx);
                }
                self.sync_params_from_url(cx);
            }
            Protocol::GraphQL => {
                self.method = HttpMethod::Post;
                self.url = req.url.clone();
                let len = self.url.chars().count();
                self.url_selection = len..len;
                if let Some(body) = &req.body
                    && let Ok(json) = serde_json::from_str::<serde_json::Value>(body)
                {
                    if let Some(query) = json.get("query").and_then(|q| q.as_str()) {
                        self.queue_editor(PendingEditor::GraphqlQuery, query.to_string());
                    }
                    if let Some(vars) = json.get("variables").filter(|v| !v.is_null()) {
                        let v = serde_json::to_string_pretty(vars).unwrap_or_default();
                        self.queue_editor(PendingEditor::GraphqlVariables, v);
                    }
                    if let Some(op) = json.get("operationName").and_then(|o| o.as_str()) {
                        self.graphql_operation_name = op.to_string();
                    }
                }
            }
            Protocol::WebSocket => {
                self.url = req.url.clone();
                let len = self.url.chars().count();
                self.url_selection = len..len;
            }
            Protocol::Grpc => {
                // The URL bar holds the server; the service and method part is
                // the method picker's, and is written back on save.
                self.url = split_grpc_url(&req.url).0.to_string();
                let len = self.url.chars().count();
                self.url_selection = len..len;
                if let Some(body) = &req.body {
                    self.queue_editor(PendingEditor::GrpcMessage, body.clone());
                }
            }
            Protocol::Trpc => {
                self.method = HttpMethod::Post;
                let url = req.url.as_str();
                if let Some(idx) = url.find("/trpc/") {
                    self.url = url[..idx + 6].to_string();
                    self.trpc_procedure = url[idx + 6..].to_string();
                } else {
                    self.url = url.to_string();
                }
                let len = self.url.chars().count();
                self.url_selection = len..len;
                if let Some(body) = &req.body {
                    self.queue_editor(PendingEditor::TrpcParams, body.clone());
                }
            }
            Protocol::SocketIO => {
                self.url = req.url.clone();
                let len = self.url.chars().count();
                self.url_selection = len..len;
            }
        }

        let pre = req.scripts.pre_script.as_deref().unwrap_or("");
        let post = req.scripts.post_script.as_deref().unwrap_or("");
        let tests = req.scripts.tests.as_deref().unwrap_or("");
        self.queue_editor(PendingEditor::PreScript, pre.to_string());
        self.queue_editor(PendingEditor::PostScript, post.to_string());
        self.queue_editor(PendingEditor::Tests, tests.to_string());
        self.pre_script = pre.to_string();
        self.post_script = post.to_string();
        self.tests = tests.to_string();

        cx.notify();
    }
}
