use super::*;

impl<E: WebSocketExecutor> RequestPanel<E> {
    /// Load request data from a history entry
    pub fn load_from_history(
        &mut self,
        method: String,
        url: String,
        headers: Vec<(String, String)>,
        body: Option<String>,
        cx: &mut Context<Self>,
    ) {
        if let Some(m) = HttpMethod::from_str(&method) {
            self.method = m;
        }

        self.url = url;
        let char_count = self.url.chars().count();
        self.url_selection = char_count..char_count;

        self.headers = headers
            .into_iter()
            .map(|(key, value)| KeyValuePair { key, value, enabled: true })
            .collect();
        if self.headers.is_empty() {
            self.headers.push(KeyValuePair::default());
        } else {
            self.headers.push(KeyValuePair::default());
        }

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

        self.headers = req.headers.iter()
            .filter(|h| h.enabled)
            .map(|h| KeyValuePair { key: h.key.clone(), value: h.value.clone(), enabled: true })
            .collect();
        if self.headers.is_empty() {
            self.headers.push(KeyValuePair::default());
        } else {
            self.headers.push(KeyValuePair::default());
        }

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
                if let Some(body) = &req.body {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
                        if let Some(query) = json.get("query").and_then(|q| q.as_str()) {
                            let q = query.to_string();
                            self.graphql_query_editor.update(cx, |ed, cx| ed.set_content(&q, cx));
                        }
                        if let Some(vars) = json.get("variables").filter(|v| !v.is_null()) {
                            let v = serde_json::to_string_pretty(vars).unwrap_or_default();
                            self.graphql_variables_editor.update(cx, |ed, cx| ed.set_content(&v, cx));
                        }
                        if let Some(op) = json.get("operationName").and_then(|o| o.as_str()) {
                            self.graphql_operation_name = op.to_string();
                        }
                    }
                }
            }
            Protocol::WebSocket => {
                self.url = req.url.clone();
                let len = self.url.chars().count();
                self.url_selection = len..len;
            }
            Protocol::Grpc => {
                let server = req.url.splitn(4, '/').take(3).collect::<Vec<_>>().join("/");
                self.url = server;
                let len = self.url.chars().count();
                self.url_selection = len..len;
                if let Some(body) = &req.body {
                    let b = body.clone();
                    self.grpc_message_editor.update(cx, |ed, cx| ed.set_content(&b, cx));
                }
            }
            Protocol::Trpc => {
                self.method = HttpMethod::Post;
                let url = req.url.as_str();
                if let Some(idx) = url.find("/trpc/") {
                    self.url = url[..idx + 5].to_string();
                    self.trpc_procedure = url[idx + 6..].to_string();
                } else {
                    self.url = url.to_string();
                }
                let len = self.url.chars().count();
                self.url_selection = len..len;
                if let Some(body) = &req.body {
                    let b = body.clone();
                    self.trpc_params_editor.update(cx, |ed, cx| ed.set_content(&b, cx));
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
        self.pre_script_editor.update(cx, |ed, cx| ed.set_content(pre, cx));
        self.post_script_editor.update(cx, |ed, cx| ed.set_content(post, cx));
        self.tests_editor.update(cx, |ed, cx| ed.set_content(tests, cx));
        self.pre_script = pre.to_string();
        self.post_script = post.to_string();
        self.tests = tests.to_string();

        cx.notify();
    }
}
