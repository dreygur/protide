use super::*;
use gpui::Context;

impl<E: WebSocketExecutor> RequestPanel<E> {
    /// Splits the gRPC message editor content into multiple messages for
    /// client-streaming/bidi calls. Messages are separated by a line
    /// containing only `---` (matching the delimiter already used to join
    /// streaming response chunks); a single-message editor with no
    /// delimiter behaves exactly as before (one message).
    fn split_grpc_messages(raw: &str) -> Vec<String> {
        let mut messages = Vec::new();
        let mut current = String::new();
        for line in raw.lines() {
            if line.trim() == "---" {
                let msg = current.trim().to_string();
                if !msg.is_empty() {
                    messages.push(msg);
                }
                current.clear();
            } else {
                current.push_str(line);
                current.push('\n');
            }
        }
        let msg = current.trim().to_string();
        if !msg.is_empty() {
            messages.push(msg);
        }
        messages
    }

    pub(super) fn spawn_grpc_server_streaming(
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
                let result = protide_core::protocols::grpc::execute_server_streaming(
                    &url,
                    &method.full_name,
                    &message,
                    metadata,
                    &proto_path,
                )
                .await;
                match result {
                    Ok(streaming) => {
                        let body = streaming.chunks.join("\n---\n");
                        let body_size = body.len();
                        let mut headers = vec![
                            (
                                "content-type".to_string(),
                                "application/grpc+json".to_string(),
                            ),
                            ("grpc-status".to_string(), "0".to_string()),
                            ("x-streaming".to_string(), "true".to_string()),
                        ];
                        let status_text = if streaming.dropped_frames > 0 {
                            headers.push((
                                "x-dropped-frames".to_string(),
                                streaming.dropped_frames.to_string(),
                            ));
                            format!(
                                "OK (streaming, {} frame(s) dropped)",
                                streaming.dropped_frames
                            )
                        } else {
                            "OK (streaming)".to_string()
                        };
                        cx.update(|cx| {
                            response_panel.update(cx, |panel, cx| {
                                panel.set_response(
                                    ResponseData {
                                        status: 200,
                                        status_text,
                                        headers,
                                        body,
                                        time: std::time::Duration::from_secs(1),
                                        size: body_size,
                                    },
                                    cx,
                                );
                            });
                        });
                    }
                    Err(e) => {
                        log::error!("gRPC streaming error: {}", e);
                        cx.update(|cx| {
                            response_panel.update(cx, |panel, cx| {
                                panel.set_response(
                                    ResponseData {
                                        status: 0,
                                        status_text: "Error".to_string(),
                                        headers: vec![],
                                        body: format!("gRPC Streaming Error: {}", e),
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

    pub(super) fn spawn_grpc_client_streaming(
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
                let messages = Self::split_grpc_messages(&message);
                let result = protide_core::protocols::grpc::execute_client_streaming(
                    &url,
                    &method.full_name,
                    messages,
                    metadata,
                    &proto_path,
                )
                .await;
                match result {
                    Ok(body) => {
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
                                        time: std::time::Duration::from_secs(1),
                                        size: body_size,
                                    },
                                    cx,
                                );
                            });
                        });
                    }
                    Err(e) => {
                        log::error!("gRPC client-streaming error: {}", e);
                        cx.update(|cx| {
                            response_panel.update(cx, |panel, cx| {
                                panel.set_response(
                                    ResponseData {
                                        status: 0,
                                        status_text: "Error".to_string(),
                                        headers: vec![],
                                        body: format!("gRPC Streaming Error: {}", e),
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

    pub(super) fn spawn_grpc_bidi(
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
                let messages = Self::split_grpc_messages(&message);
                let result = protide_core::protocols::grpc::execute_bidi_streaming(
                    &url,
                    &method.full_name,
                    messages,
                    metadata,
                    &proto_path,
                )
                .await;
                match result {
                    Ok(streaming) => {
                        let body = streaming.chunks.join("\n---\n");
                        let body_size = body.len();
                        let mut headers = vec![
                            (
                                "content-type".to_string(),
                                "application/grpc+json".to_string(),
                            ),
                            ("grpc-status".to_string(), "0".to_string()),
                            ("x-streaming".to_string(), "true".to_string()),
                        ];
                        let status_text = if streaming.dropped_frames > 0 {
                            headers.push((
                                "x-dropped-frames".to_string(),
                                streaming.dropped_frames.to_string(),
                            ));
                            format!("OK (bidi, {} frame(s) dropped)", streaming.dropped_frames)
                        } else {
                            "OK (bidi)".to_string()
                        };
                        cx.update(|cx| {
                            response_panel.update(cx, |panel, cx| {
                                panel.set_response(
                                    ResponseData {
                                        status: 200,
                                        status_text,
                                        headers,
                                        body,
                                        time: std::time::Duration::from_secs(1),
                                        size: body_size,
                                    },
                                    cx,
                                );
                            });
                        });
                    }
                    Err(e) => {
                        log::error!("gRPC bidi-streaming error: {}", e);
                        cx.update(|cx| {
                            response_panel.update(cx, |panel, cx| {
                                panel.set_response(
                                    ResponseData {
                                        status: 0,
                                        status_text: "Error".to_string(),
                                        headers: vec![],
                                        body: format!("gRPC Bidi Error: {}", e),
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
