use super::*;

impl<E: WebSocketExecutor> RequestPanel<E> {
    /// Capture a serialisable snapshot of the current editor state.
    pub fn capture_draft(&self, cx: &gpui::App) -> crate::session::RequestDraft {
        use crate::session::{HeaderEntry, RequestDraft};
        use AuthType::*;
        use ApiKeyLocation::*;

        RequestDraft {
            protocol: match self.request_mode {
                RequestMode::Http      => "http",
                RequestMode::GraphQL   => "graphql",
                RequestMode::WebSocket => "websocket",
                RequestMode::Grpc      => "grpc",
                RequestMode::Trpc      => "trpc",
                RequestMode::SocketIo  => "socketio",
            }.to_string(),
            active_tab: self.active_tab,
            url: self.url.clone(),
            method: self.method.as_str().to_string(),
            headers: self.headers.iter()
                .filter(|h| !h.key.is_empty())
                .map(|h| HeaderEntry { key: h.key.clone(), value: h.value.clone(), enabled: h.enabled })
                .collect(),
            body: self.body_editor.read(cx).content().to_string(),
            body_type: match self.body_type {
                BodyType::Json   => "json",
                BodyType::Xml    => "xml",
                BodyType::Raw    => "raw",
                BodyType::Form   => "form",
                BodyType::Binary => "binary",
            }.to_string(),
            auth_type: match self.auth_type {
                None       => "none",
                Bearer     => "bearer",
                Basic      => "basic",
                ApiKey     => "apikey",
                ClientCert => "client_cert",
            }.to_string(),
            bearer_token:    self.bearer_token.clone(),
            basic_username:  self.basic_username.clone(),
            basic_password:  self.basic_password.clone(),
            api_key_name:    self.api_key_name.clone(),
            api_key_value:   self.api_key_value.clone(),
            api_key_location: match self.api_key_location {
                Header     => "header",
                QueryParam => "query",
            }.to_string(),
            graphql_query:          self.graphql_query_editor.read(cx).content().to_string(),
            graphql_variables:      self.graphql_variables_editor.read(cx).content().to_string(),
            graphql_operation_name: self.graphql_operation_name.clone(),
            grpc_message:    self.grpc_message_editor.read(cx).content().to_string(),
            grpc_proto_path: self.grpc_proto_path.clone(),
            grpc_service:    self.grpc_service.clone(),
            grpc_method_name: self.grpc_method.as_ref().map(|m| m.full_name.clone()),
            trpc_procedure:  self.trpc_procedure.clone(),
            trpc_params:     self.trpc_params_editor.read(cx).content().to_string(),
            trpc_batch_calls: self.trpc_batch_calls.iter()
                .map(|c| crate::session::TrpcBatchCallDraft {
                    procedure: c.procedure.clone(),
                    params: c.params.clone(),
                    enabled: c.enabled,
                })
                .collect(),
            trpc_selected_batch_idx: self.trpc_selected_batch_idx,
            sio_namespace:   self.sio_namespace.clone(),
            sio_event_name:  self.sio_event_name.clone(),
            sio_payload:     self.sio_payload_editor.read(cx).content().to_string(),
        }
    }

    /// Restore editor state from a previously captured draft.
    pub fn restore_from_draft(&mut self, draft: &crate::session::RequestDraft, cx: &mut Context<Self>) {
        // Switch protocol mode
        self.request_mode = match draft.protocol.as_str() {
            "graphql"   => RequestMode::GraphQL,
            "websocket" => RequestMode::WebSocket,
            "grpc"      => RequestMode::Grpc,
            "trpc"      => RequestMode::Trpc,
            "socketio"  => RequestMode::SocketIo,
            _           => RequestMode::Http,
        };
        self.active_tab = draft.active_tab;
        self.active_edit = Option::None;
        self.method_dropdown_open = false;
        self.variable_extractions.clear();

        // Method + URL
        if let Some(m) = HttpMethod::from_str(&draft.method) {
            self.method = m;
        }
        self.url = draft.url.clone();
        let len = self.url.chars().count();
        self.url_selection = len..len;

        // Headers
        self.headers = draft.headers.iter()
            .map(|h| KeyValuePair { key: h.key.clone(), value: h.value.clone(), enabled: h.enabled })
            .collect();
        if self.headers.is_empty() {
            self.headers.push(KeyValuePair::default());
        } else {
            self.headers.push(KeyValuePair::default());
        }

        // Body
        self.body_type = match draft.body_type.as_str() {
            "xml"    => BodyType::Xml,
            "raw"    => BodyType::Raw,
            "form"   => BodyType::Form,
            "binary" => BodyType::Binary,
            _        => BodyType::Json,
        };
        if !draft.body.is_empty() {
            let b = draft.body.clone();
            self.body_editor.update(cx, |ed, cx| ed.set_content(&b, cx));
        }

        // Auth
        self.auth_type = match draft.auth_type.as_str() {
            "bearer"      => AuthType::Bearer,
            "basic"       => AuthType::Basic,
            "apikey"      => AuthType::ApiKey,
            "client_cert" => AuthType::ClientCert,
            _        => AuthType::None,
        };
        self.bearer_token   = draft.bearer_token.clone();
        self.basic_username = draft.basic_username.clone();
        self.basic_password = draft.basic_password.clone();
        self.api_key_name   = draft.api_key_name.clone();
        self.api_key_value  = draft.api_key_value.clone();
        self.api_key_location = match draft.api_key_location.as_str() {
            "query" => ApiKeyLocation::QueryParam,
            _       => ApiKeyLocation::Header,
        };

        // GraphQL
        if !draft.graphql_query.is_empty() {
            let q = draft.graphql_query.clone();
            self.graphql_query_editor.update(cx, |ed, cx| ed.set_content(&q, cx));
        }
        if !draft.graphql_variables.is_empty() {
            let v = draft.graphql_variables.clone();
            self.graphql_variables_editor.update(cx, |ed, cx| ed.set_content(&v, cx));
        }
        self.graphql_operation_name = draft.graphql_operation_name.clone();

        // gRPC
        if !draft.grpc_message.is_empty() {
            let m = draft.grpc_message.clone();
            self.grpc_message_editor.update(cx, |ed, cx| ed.set_content(&m, cx));
        }
        if let Some(ref proto_path) = draft.grpc_proto_path {
            self.load_grpc_proto_from_path(proto_path.clone(), cx);
            if let Some(ref svc) = draft.grpc_service {
                if self.grpc_services.contains(svc) {
                    self.grpc_service = Some(svc.clone());
                    self.grpc_methods.retain(|m| m.full_name.starts_with(svc.as_str()));
                }
            }
            if let Some(ref method_name) = draft.grpc_method_name {
                if let Some(m) = self.grpc_methods.iter().find(|m| &m.full_name == method_name) {
                    self.grpc_method = Some(m.clone());
                }
            }
        }

        // tRPC
        self.trpc_procedure = draft.trpc_procedure.clone();
        if !draft.trpc_params.is_empty() {
            let p = draft.trpc_params.clone();
            self.trpc_params_editor.update(cx, |ed, cx| ed.set_content(&p, cx));
        }
        self.trpc_batch_calls = draft.trpc_batch_calls.iter()
            .map(|c| TrpcBatchCall { procedure: c.procedure.clone(), params: c.params.clone(), enabled: c.enabled })
            .collect();
        if self.trpc_batch_calls.is_empty() {
            self.trpc_batch_calls.push(TrpcBatchCall { enabled: true, ..Default::default() });
        }
        self.trpc_selected_batch_idx = draft.trpc_selected_batch_idx;
        if let Some(idx) = self.trpc_selected_batch_idx {
            if let Some(call) = self.trpc_batch_calls.get(idx) {
                let p = call.params.clone();
                self.trpc_batch_params_editor.update(cx, |ed, cx| ed.set_content(&p, cx));
            }
        }

        // Socket.IO
        self.sio_namespace  = draft.sio_namespace.clone();
        self.sio_event_name = draft.sio_event_name.clone();
        if !draft.sio_payload.is_empty() {
            let p = draft.sio_payload.clone();
            self.sio_payload_editor.update(cx, |ed, cx| ed.set_content(&p, cx));
        }

        self.sync_params_from_url(cx);
        cx.notify();
    }
}
