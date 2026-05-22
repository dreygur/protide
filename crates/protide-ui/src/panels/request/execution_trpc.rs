use gpui::Context;
use super::*;
use super::super::request_types::PendingEditor;
use super::super::request_utils::status_text;

impl<E: WebSocketExecutor> RequestPanel<E> {
    pub(super) fn send_trpc_request(&mut self, cx: &mut Context<Self>) {
        if self.trpc_procedure.trim().is_empty() { return; }

        self.loading = true;
        cx.notify();

        let url = self.url.clone();
        let procedure = self.trpc_procedure.clone();
        let params = self.trpc_params_editor.read(cx).value().to_string();

        let env_state = self.explorer_panel.as_ref().map(|p| p.read(cx).env_state().clone());
        let substitute = |s: &str| -> String {
            if let Some(ref env) = env_state { env.substitute(s) } else { s.to_string() }
        };

        let url = substitute(&url);
        let procedure = substitute(&procedure);

        let mut headers: Vec<(String, String)> = self.headers.iter()
            .filter(|h| h.enabled && !h.key.is_empty())
            .map(|h| (substitute(&h.key), substitute(&h.value)))
            .collect();

        match self.auth_type {
            AuthType::Bearer => {
                if !self.bearer_token.is_empty() {
                    let token = substitute(&self.bearer_token);
                    headers.push(("Authorization".to_string(), format!("Bearer {}", token)));
                }
            }
            AuthType::Basic => {
                if !self.basic_username.is_empty() {
                    let username = substitute(&self.basic_username);
                    let password = substitute(&self.basic_password);
                    let credentials = base64::engine::general_purpose::STANDARD
                        .encode(format!("{}:{}", username, password));
                    headers.push(("Authorization".to_string(), format!("Basic {}", credentials)));
                }
            }
            AuthType::ApiKey => {
                if !self.api_key_name.is_empty() {
                    let key_name = substitute(&self.api_key_name);
                    let key_value = substitute(&self.api_key_value);
                    if self.api_key_location == ApiKeyLocation::Header {
                        headers.push((key_name, key_value));
                    }
                }
            }
            AuthType::None => {}
        }

        let response_panel = self.response_panel.clone();
        log::info!("tRPC {} {}", url, procedure);

        cx.spawn(async move |this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let result = protide_core::protocols::trpc::execute_trpc(&url, &procedure, &params, headers);
                let _ = tx.send(result);
            });

            match rx.recv_timeout(std::time::Duration::from_secs(30)) {
                Ok(Ok((body, elapsed, status_code))) => {
                    let body_size = body.len();
                    let _ = cx.update(|cx| {
                        response_panel.update(cx, |panel, cx| {
                            panel.set_response(ResponseData {
                                status: status_code,
                                status_text: status_text(status_code).to_string(),
                                headers: vec![("content-type".to_string(), "application/json".to_string())],
                                body,
                                time: elapsed,
                                size: body_size,
                            }, cx);
                        });
                    });
                }
                Ok(Err(e)) => {
                    log::error!("tRPC error: {}", e);
                    let _ = cx.update(|cx| {
                        response_panel.update(cx, |panel, cx| {
                            let error_body = serde_json::json!({ "error": e }).to_string();
                            panel.set_response(ResponseData {
                                status: 500,
                                status_text: "tRPC Error".to_string(),
                                headers: vec![("content-type".to_string(), "application/json".to_string())],
                                body: error_body.clone(),
                                time: std::time::Duration::from_secs(0),
                                size: error_body.len(),
                            }, cx);
                        });
                    });
                }
                Err(_) => {
                    log::error!("tRPC request timed out");
                    let _ = cx.update(|cx| {
                        response_panel.update(cx, |panel, cx| {
                            panel.set_error("tRPC request timed out (30s)".to_string(), cx);
                        });
                    });
                }
            }
            let _ = cx.update(|cx| { let _ = this.update(cx, |p, cx| { p.loading = false; cx.notify(); }); });
        }).detach();
    }

    pub(super) fn run_trpc_playground(&mut self, cx: &mut Context<Self>) {
        let idx = match self.trpc_pg_selected { Some(i) => i, None => return };
        let proc = match self.trpc_pg_procedures.get(idx) { Some(p) => p.clone(), None => return };

        self.trpc_pg_loading = true;
        self.trpc_pg_response = None;
        self.trpc_pg_error = None;
        self.trpc_pg_status = None;
        self.trpc_pg_elapsed = None;
        cx.notify();

        let url = self.url.clone();
        let procedure = proc.full_procedure();
        let params = self.trpc_params_editor.read(cx).value().to_string();

        let env_state = self.explorer_panel.as_ref().map(|p| p.read(cx).env_state().clone());
        let substitute = move |s: &str| -> String {
            if let Some(ref env) = env_state { env.substitute(s) } else { s.to_string() }
        };
        let url = substitute(&url);
        let procedure = substitute(&procedure);

        let mut headers: Vec<(String, String)> = self.headers.iter()
            .filter(|h| h.enabled && !h.key.is_empty())
            .map(|h| (substitute(&h.key), substitute(&h.value)))
            .collect();

        match self.auth_type {
            AuthType::Bearer if !self.bearer_token.is_empty() => {
                let token = substitute(&self.bearer_token);
                headers.push(("Authorization".to_string(), format!("Bearer {}", token)));
            }
            AuthType::Basic if !self.basic_username.is_empty() => {
                let username = substitute(&self.basic_username);
                let password = substitute(&self.basic_password);
                let cred = base64::engine::general_purpose::STANDARD
                    .encode(format!("{}:{}", username, password));
                headers.push(("Authorization".to_string(), format!("Basic {}", cred)));
            }
            AuthType::ApiKey if !self.api_key_name.is_empty() && self.api_key_location == ApiKeyLocation::Header => {
                headers.push((substitute(&self.api_key_name), substitute(&self.api_key_value)));
            }
            _ => {}
        }

        log::info!("tRPC playground: {} {}", url, procedure);

        cx.spawn(async move |this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let _ = tx.send(protide_core::protocols::trpc::execute_trpc(&url, &procedure, &params, headers));
            });

            let result = rx.recv_timeout(std::time::Duration::from_secs(30));
            let _ = cx.update(|cx| {
                let _ = this.update(cx, |panel, cx| {
                    panel.trpc_pg_loading = false;
                    match result {
                        Ok(Ok((body, elapsed, status))) => {
                            panel.trpc_pg_status = Some(status);
                            panel.trpc_pg_elapsed = Some(elapsed);
                            panel.trpc_pg_response = Some(body.clone());
                            panel.queue_editor(PendingEditor::TrpcPgResult, body);
                        }
                        Ok(Err(e)) => {
                            panel.trpc_pg_status = Some(500);
                            panel.trpc_pg_error = Some(e.clone());
                            let val = serde_json::json!({ "error": e });
                            let body = serde_json::to_string_pretty(&val)
                                .unwrap_or_else(|_| e.clone());
                            panel.queue_editor(PendingEditor::TrpcPgResult, body);
                        }
                        Err(_) => {
                            let msg = "Request timed out (30s)";
                            panel.trpc_pg_error = Some(msg.to_string());
                            let val = serde_json::json!({ "error": msg });
                            let body = serde_json::to_string_pretty(&val)
                                .unwrap_or_else(|_| msg.to_string());
                            panel.queue_editor(PendingEditor::TrpcPgResult, body);
                        }
                    }
                    cx.notify();
                });
            });
        }).detach();
    }
}
