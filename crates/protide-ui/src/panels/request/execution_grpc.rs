use super::*;
use gpui::Context;

impl<E: WebSocketExecutor> RequestPanel<E> {
    pub(super) fn toggle_grpc_service_picker(&mut self, cx: &mut Context<Self>) {
        self.grpc_service_picker_open = !self.grpc_service_picker_open;
        self.grpc_method_picker_open = false;
        cx.notify();
    }

    pub(super) fn toggle_grpc_method_picker(&mut self, cx: &mut Context<Self>) {
        if self.grpc_service.is_none() {
            return;
        }
        self.grpc_method_picker_open = !self.grpc_method_picker_open;
        self.grpc_service_picker_open = false;
        cx.notify();
    }

    pub(super) fn select_grpc_service(&mut self, service: String, cx: &mut Context<Self>) {
        let prefix = format!("{}/", service);
        self.grpc_method = self
            .grpc_methods
            .iter()
            .find(|m| m.full_name.starts_with(&prefix))
            .cloned();
        self.grpc_service = Some(service);
        self.grpc_service_picker_open = false;
        cx.notify();
    }

    pub(super) fn select_grpc_method(&mut self, method: GrpcMethodInfo, cx: &mut Context<Self>) {
        self.grpc_method = Some(method);
        self.grpc_method_picker_open = false;
        cx.notify();
    }

    pub(super) fn send_grpc_request(&mut self, cx: &mut Context<Self>) {
        let Some(method) = &self.grpc_method else {
            return;
        };
        let Some(proto_path) = self.grpc_proto_path.clone() else {
            return;
        };

        self.loading = true;
        cx.notify();

        let message = self.grpc_message_editor.read(cx).value().to_string();
        let url = self.url.clone();
        let method = method.clone();
        let streaming_type = method.streaming_type;

        let env_state = self
            .explorer_panel
            .as_ref()
            .map(|p| p.read(cx).env_state().clone());
        let substitute = |s: &str| -> String {
            if let Some(ref env) = env_state {
                env.substitute(s)
            } else {
                s.to_string()
            }
        };
        let url = substitute(&url);
        let metadata: Vec<(String, String)> = self
            .grpc_metadata
            .iter()
            .filter(|m| m.enabled && !m.key.is_empty())
            .map(|m| (substitute(&m.key), substitute(&m.value)))
            .collect();
        let response_panel = self.response_panel.clone();
        log::info!("gRPC {} {}", url, method.full_name);

        match streaming_type {
            GrpcStreamingType::Unary => self.spawn_grpc_unary(
                url,
                method,
                message,
                metadata,
                proto_path,
                response_panel,
                cx,
            ),
            GrpcStreamingType::ServerStreaming => self.spawn_grpc_server_streaming(
                url,
                method,
                message,
                metadata,
                proto_path,
                response_panel,
                cx,
            ),
            GrpcStreamingType::ClientStreaming => self.spawn_grpc_client_streaming(
                url,
                method,
                message,
                metadata,
                proto_path,
                response_panel,
                cx,
            ),
            GrpcStreamingType::BidiStreaming => self.spawn_grpc_bidi(
                url,
                method,
                message,
                metadata,
                proto_path,
                response_panel,
                cx,
            ),
        }
    }

    fn spawn_grpc_unary(
        &mut self,
        url: String,
        method: GrpcMethodInfo,
        message: String,
        metadata: Vec<(String, String)>,
        proto_path: std::path::PathBuf,
        response_panel: gpui::Entity<ResponsePanel>,
        cx: &mut Context<Self>,
    ) {
        cx.spawn(
            async move |this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
                let result = run_blocking(cx, move || {
                    protide_core::protocols::grpc::execute_unary_blocking(
                        &url,
                        &method.full_name,
                        &message,
                        metadata,
                        &proto_path,
                    )
                })
                .await
                .unwrap_or_else(|_| Err("gRPC thread panicked".to_string()));
                match result {
                    Ok((body, elapsed)) => {
                        let body_size = body.len();
                        cx.update(|cx| {
                            response_panel.update(cx, |panel, cx| {
                                panel.set_response(
                                    ResponseData {
                                        status: 200,
                                        status_text: "OK".to_string(),
                                        headers: vec![
                                            (
                                                "content-type".to_string(),
                                                "application/grpc+json".to_string(),
                                            ),
                                            ("grpc-status".to_string(), "0".to_string()),
                                        ],
                                        body,
                                        time: elapsed,
                                        size: body_size,
                                    },
                                    cx,
                                );
                            });
                        });
                    }
                    Err(e) => {
                        // covers both gRPC errors and thread panics; gRPC library deadline handles timeouts
                        log::error!("gRPC error: {}", e);
                        cx.update(|cx| {
                            response_panel.update(cx, |panel, cx| {
                                panel.set_response(
                                    ResponseData {
                                        status: 0,
                                        status_text: "Error".to_string(),
                                        headers: vec![],
                                        body: format!("gRPC Error: {}", e),
                                        time: std::time::Duration::ZERO,
                                        size: 0,
                                    },
                                    cx,
                                );
                            });
                        });
                    }
                }
                cx.update(|cx| {
                    let _ = this.update(cx, |p, cx| {
                        p.loading = false;
                        cx.notify();
                    });
                });
            },
        )
        .detach();
    }
}
