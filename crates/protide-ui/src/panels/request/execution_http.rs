use gpui::Context;
use super::*;
use super::super::request_utils::{base64_encode, url_encode};
use super::graphql::dns_troubleshoot_hint;

impl<E: WebSocketExecutor> RequestPanel<E> {
    pub fn send_request(&mut self, cx: &mut Context<Self>) {
        if self.loading || self.url.is_empty() { return; }
        self.loading = true;
        cx.notify();

        let body_content = self.body_editor.read(cx).value().to_string();
        let is_graphql_mode = self.request_mode == RequestMode::GraphQL;
        let graphql_query = if is_graphql_mode { self.graphql_query_editor.read(cx).value().to_string() } else { String::new() };
        let graphql_variables = if is_graphql_mode { self.graphql_variables_editor.read(cx).value().to_string() } else { String::new() };
        let pre_script = self.pre_script_editor.read(cx).value().to_string();
        let post_script = self.post_script_editor.read(cx).value().to_string();
        let tests_script = self.tests_editor.read(cx).value().to_string();

        self.response_panel.update(cx, |panel, cx| panel.set_loading(cx));

        let env_state = self.explorer_panel.as_ref().map(|panel| panel.read(cx).env_state().clone());
        let substitute = |s: &str| -> String {
            if let Some(ref env) = env_state { env.substitute(s) } else { s.to_string() }
        };

        let url = substitute(&self.url);
        let method = self.method.clone();
        let response_panel = self.response_panel.clone();
        let variable_extractions = self.variable_extractions.clone();
        let explorer_panel = self.explorer_panel.clone();

        let mut headers: Vec<(String, String)> = self.headers.iter()
            .filter(|h| h.enabled && !h.key.is_empty())
            .map(|h| (substitute(&h.key), substitute(&h.value)))
            .collect();

        let auth_type = self.auth_type;
        let bearer_token = substitute(&self.bearer_token);
        let basic_username = substitute(&self.basic_username);
        let basic_password = substitute(&self.basic_password);
        let api_key_name = substitute(&self.api_key_name);
        let api_key_value = substitute(&self.api_key_value);
        let api_key_location = self.api_key_location;

        match auth_type {
            AuthType::None => {}
            AuthType::Bearer => {
                if !bearer_token.is_empty() {
                    headers.push(("Authorization".to_string(), format!("Bearer {}", bearer_token)));
                }
            }
            AuthType::Basic => {
                if !basic_username.is_empty() || !basic_password.is_empty() {
                    let credentials = format!("{}:{}", basic_username, basic_password);
                    let encoded = base64_encode(credentials.as_bytes());
                    headers.push(("Authorization".to_string(), format!("Basic {}", encoded)));
                }
            }
            AuthType::ApiKey => {
                if !api_key_name.is_empty() && !api_key_value.is_empty() {
                    if api_key_location == ApiKeyLocation::Header {
                        headers.push((api_key_name.clone(), api_key_value.clone()));
                    }
                }
            }
        }

        let binary_file_path = self.binary_file_path.clone();
        let has_files = self.body_type == BodyType::Form
            && self.form_data.iter().any(|f| f.enabled && f.field_type == FormFieldType::File && f.file_path.is_some());

        let form_fields: Vec<(String, String, Option<std::path::PathBuf>, bool)> = if self.body_type == BodyType::Form {
            self.form_data.iter()
                .filter(|f| f.enabled && !f.key.is_empty())
                .map(|f| (substitute(&f.key), substitute(&f.value), f.file_path.clone(), f.field_type == FormFieldType::File))
                .collect()
        } else {
            Vec::new()
        };

        let exec_body = if is_graphql_mode {
            ExecutionBody::None
        } else if matches!(method, HttpMethod::Post | HttpMethod::Put | HttpMethod::Patch | HttpMethod::Custom(_)) {
            match self.body_type {
                BodyType::Form if !has_files => {
                    let s = form_fields.iter()
                        .filter(|(_, _, _, is_file)| !is_file)
                        .map(|(k, v, _, _)| format!("{}={}", url_encode(k), url_encode(v)))
                        .collect::<Vec<_>>().join("&");
                    if s.is_empty() { ExecutionBody::None } else { ExecutionBody::Text(s) }
                }
                BodyType::Form => ExecutionBody::Multipart(
                    form_fields.iter().map(|(k, v, path, is_file)| FormPart {
                        name: k.clone(),
                        value: if *is_file {
                            FormPartValue::File(path.clone().unwrap_or_default())
                        } else {
                            FormPartValue::Text(v.clone())
                        },
                    }).collect(),
                ),
                BodyType::Binary => binary_file_path.as_ref()
                    .and_then(|p| std::fs::read(p).ok())
                    .map(ExecutionBody::Binary)
                    .unwrap_or(ExecutionBody::None),
                _ => ExecutionBody::Text(substitute(&body_content)),
            }
        } else {
            ExecutionBody::None
        };

        let exec_mode = if is_graphql_mode {
            ExecutionMode::GraphQL {
                query: substitute(&graphql_query),
                variables: substitute(&graphql_variables),
                operation_name: if self.graphql_operation_name.trim().is_empty() {
                    None
                } else {
                    Some(substitute(&self.graphql_operation_name))
                },
            }
        } else {
            ExecutionMode::Http
        };

        let env_vars: std::collections::HashMap<String, String> = env_state.as_ref()
            .and_then(|e| e.active())
            .map(|env| env.variables.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default();

        let final_url = if auth_type == AuthType::ApiKey
            && api_key_location == ApiKeyLocation::QueryParam
            && !api_key_name.is_empty()
            && !api_key_value.is_empty()
        {
            if url.contains('?') { format!("{}&{}={}", url, url_encode(&api_key_name), url_encode(&api_key_value)) }
            else { format!("{}?{}={}", url, url_encode(&api_key_name), url_encode(&api_key_value)) }
        } else {
            url
        };

        let history_badge = match self.request_mode {
            RequestMode::Http => method.as_str().to_string(),
            RequestMode::GraphQL => "GQL".to_string(),
            RequestMode::WebSocket => "WS".to_string(),
            RequestMode::Grpc => "GRPC".to_string(),
            RequestMode::Trpc => "TRPC".to_string(),
            RequestMode::SocketIo => "SIO".to_string(),
        };
        let history_id = cx.update_global::<super::super::history::RequestHistory, _>(|history, _| {
            history.add(history_badge, final_url.clone(), headers.clone(), exec_body.as_text())
        });

        let console_panel = self.console_panel.clone();
        let log_protocol = match self.request_mode {
            RequestMode::Http     => "HTTP",
            RequestMode::GraphQL  => "GraphQL",
            RequestMode::WebSocket => "WebSocket",
            RequestMode::Grpc     => "gRPC",
            RequestMode::Trpc     => "tRPC",
            RequestMode::SocketIo => "Socket.IO",
        };
        let log_method = method.as_str().to_string();
        let log_url = final_url.clone();
        log::info!("[{}] → {} {}", log_protocol, log_method, log_url);

        let timeout_secs: u64 = self.timeout_input.read(cx).value().to_string().trim().parse().unwrap_or(30);
        let verify_ssl = self.verify_ssl;
        let impersonate_browser = self.impersonate_browser;
        let req = ExecutionRequest {
            method: method.as_str().to_string(),
            url: final_url.clone(),
            headers,
            body: exec_body,
            mode: exec_mode,
            pre_script,
            post_script,
            tests: tests_script,
            env_vars,
            variable_extractions,
            timeout_secs,
            verify_ssl,
            impersonate_browser,
        };

        cx.spawn(async move |this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
            let result = std::thread::spawn(move || protide_core::execution::execute(req))
                .join()
                .unwrap_or_else(|_| Err("Request thread panicked".to_string()));

            match result {
                Ok(data) => {
                    log::info!("[{}] ← {} {} in {}ms", log_protocol, data.status, data.status_text, data.time.as_millis());
                    let _ = cx.update(|cx| {
                        cx.update_global::<super::super::history::RequestHistory, _>(|history, _| {
                            history.update_response(history_id, data.status, data.time);
                        });
                        if !data.extracted_vars.is_empty() || !data.env_changes.is_empty() {
                            if let Some(explorer) = &explorer_panel {
                                explorer.update(cx, |panel, cx| {
                                    for (name, value) in data.extracted_vars.iter().chain(data.env_changes.iter()) {
                                        panel.set_env_variable(name, value, cx);
                                    }
                                });
                            }
                        }
                        if let Some(console) = &console_panel {
                            let duration_ms = data.time.as_millis() as u64;
                            let status = data.status;
                            let body_preview = data.body.clone();
                            console.update(cx, |panel, cx| {
                                panel.log(ConsoleEntry {
                                    timestamp: chrono::Local::now(),
                                    level: LogLevel::Info,
                                    source: ConsoleEntrySource::Request,
                                    protocol: log_protocol.to_string(),
                                    method: log_method.clone(),
                                    url: log_url.clone(),
                                    status,
                                    duration_ms,
                                    error: None,
                                    response_body: body_preview,
                                    troubleshoot_hint: None,
                                }, cx);
                            });
                        }
                        response_panel.update(cx, |panel, cx| {
                            panel.set_response(ResponseData {
                                status: data.status,
                                status_text: data.status_text,
                                headers: data.headers,
                                body: data.body,
                                time: data.time,
                                size: data.size,
                            }, cx);
                            if !data.test_results.is_empty() {
                                panel.set_test_results(data.test_results, cx);
                            }
                        });
                    });
                }
                Err(e) => {
                    log::error!("[{}] Request failed {}: {}", log_protocol, log_url, e);
                    let _ = cx.update(|cx| {
                        if let Some(console) = &console_panel {
                            let err = e.clone();
                            let hint = dns_troubleshoot_hint(&err);
                            console.update(cx, |panel, cx| {
                                panel.log(ConsoleEntry {
                                    timestamp: chrono::Local::now(),
                                    level: LogLevel::Error,
                                    source: ConsoleEntrySource::Request,
                                    protocol: log_protocol.to_string(),
                                    method: log_method.clone(),
                                    url: log_url.clone(),
                                    status: 0,
                                    duration_ms: 0,
                                    error: Some(err),
                                    response_body: String::new(),
                                    troubleshoot_hint: hint,
                                }, cx);
                            });
                        }
                        response_panel.update(cx, |panel, cx| { panel.set_error(e, cx); });
                    });
                }
            }
            let _ = cx.update(|cx| { let _ = this.update(cx, |this, cx| { this.loading = false; cx.notify(); }); });
        }).detach();
    }
}

// suppress unused import
#[allow(unused_imports)]
use super::super::request_utils::status_text as _status_text_unused;
