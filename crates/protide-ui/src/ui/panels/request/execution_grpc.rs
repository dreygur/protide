use gpui::Context;
use super::*;

impl<E: WebSocketExecutor> RequestPanel<E> {
    pub(super) fn load_proto_file(&mut self, cx: &mut Context<Self>) {
        use rfd::FileDialog;
        let mut dialog = FileDialog::new()
            .add_filter("Proto Files", &["proto"])
            .set_title("Select Proto File");
        if let Some(dir) = last_paths::last_dir("proto_file") {
            dialog = dialog.set_directory(dir);
        }
        let path = dialog.pick_file();
        if let Some(path) = path {
            last_paths::save_last_dir("proto_file", &path);
            match std::fs::read_to_string(&path) {
                Ok(content) => {
                    self.grpc_proto_path = Some(path);
                    self.grpc_proto_content = content.clone();
                    self.parse_proto_services(&content);
                    log::info!("Proto loaded: {} ({} services)", self.grpc_proto_path.as_ref().unwrap().display(), self.grpc_services.len());
                    cx.notify();
                }
                Err(e) => { log::error!("Failed to read proto file: {}", e); }
            }
        }
    }

    pub fn load_grpc_proto_from_path(&mut self, path: std::path::PathBuf, cx: &mut Context<Self>) {
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                self.grpc_proto_path = Some(path);
                self.grpc_proto_content = content.clone();
                self.parse_proto_services(&content);
                log::info!("Proto loaded: {} ({} services)", self.grpc_proto_path.as_ref().unwrap().display(), self.grpc_services.len());
                cx.notify();
            }
            Err(e) => { log::error!("Failed to read proto file: {}", e); }
        }
    }

    pub(super) fn parse_proto_services(&mut self, content: &str) {
        self.grpc_services.clear();
        self.grpc_methods.clear();
        self.grpc_service = None;
        self.grpc_method = None;

        if let Some(ref path) = self.grpc_proto_path.clone() {
            if let Ok(pool) = protide_core::protocols::grpc::parse_proto_file(path) {
                for svc in pool.services() {
                    let svc_name = svc.full_name().to_string();
                    self.grpc_services.push(svc_name.clone());
                    for method in svc.methods() {
                        let streaming_type = match (method.is_client_streaming(), method.is_server_streaming()) {
                            (false, false) => GrpcStreamingType::Unary,
                            (false, true) => GrpcStreamingType::ServerStreaming,
                            (true, false) => GrpcStreamingType::ClientStreaming,
                            (true, true) => GrpcStreamingType::BidiStreaming,
                        };
                        self.grpc_methods.push(GrpcMethodInfo {
                            full_name: format!("{}/{}", svc_name, method.name()),
                            streaming_type,
                        });
                    }
                }
                if let Some(s) = self.grpc_services.first() { self.grpc_service = Some(s.clone()); }
                if let Some(m) = self.grpc_methods.first() { self.grpc_method = Some(m.clone()); }
                return;
            }
        }

        // Fallback: basic text parsing
        let mut in_service = false;
        let mut current_service = String::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("service ") {
                if let Some(name) = trimmed.strip_prefix("service ").and_then(|s| s.split_whitespace().next()) {
                    current_service = name.to_string();
                    self.grpc_services.push(current_service.clone());
                    in_service = true;
                }
            }
            if in_service && trimmed.starts_with("rpc ") {
                if let Some(name) = trimmed.strip_prefix("rpc ").and_then(|s| s.split('(').next()).map(|s| s.trim()) {
                    self.grpc_methods.push(GrpcMethodInfo {
                        full_name: format!("{}/{}", current_service, name),
                        streaming_type: GrpcStreamingType::Unary,
                    });
                }
            }
            if in_service && trimmed == "}" { in_service = false; }
        }
        if let Some(s) = self.grpc_services.first() { self.grpc_service = Some(s.clone()); }
        if let Some(m) = self.grpc_methods.first() { self.grpc_method = Some(m.clone()); }
    }

    pub(super) fn send_grpc_request(&mut self, cx: &mut Context<Self>) {
        let Some(method) = &self.grpc_method else { return; };
        let Some(proto_path) = self.grpc_proto_path.clone() else { return; };

        self.loading = true;
        cx.notify();

        let message = self.grpc_message_editor.read(cx).content().to_string();
        let url = self.url.clone();
        let method = method.clone();
        let streaming_type = method.streaming_type;

        let env_state = self.explorer_panel.as_ref().map(|p| p.read(cx).env_state().clone());
        let substitute = |s: &str| -> String {
            if let Some(ref env) = env_state { env.substitute(s) } else { s.to_string() }
        };
        let url = substitute(&url);
        let metadata: Vec<(String, String)> = self.grpc_metadata.iter()
            .filter(|m| m.enabled && !m.key.is_empty())
            .map(|m| (substitute(&m.key), substitute(&m.value)))
            .collect();
        let response_panel = self.response_panel.clone();
        log::info!("gRPC {} {}", url, method.full_name);

        match streaming_type {
            GrpcStreamingType::Unary => self.spawn_grpc_unary(url, method, message, metadata, proto_path, response_panel, cx),
            GrpcStreamingType::ServerStreaming => self.spawn_grpc_server_streaming(url, method, message, metadata, proto_path, response_panel, cx),
            GrpcStreamingType::ClientStreaming => self.spawn_grpc_client_streaming(url, method, message, metadata, proto_path, response_panel, cx),
            GrpcStreamingType::BidiStreaming => self.spawn_grpc_bidi(url, method, message, metadata, proto_path, response_panel, cx),
        }
    }

    fn spawn_grpc_unary(
        &mut self,
        url: String, method: GrpcMethodInfo, message: String,
        metadata: Vec<(String, String)>, proto_path: std::path::PathBuf,
        response_panel: gpui::Entity<ResponsePanel>,
        cx: &mut Context<Self>,
    ) {
        cx.spawn(async move |this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
            let (tx, rx) = std::sync::mpsc::channel::<Result<(String, std::time::Duration), String>>();
            std::thread::spawn(move || {
                let result = protide_core::protocols::grpc::execute_unary_blocking(&url, &method.full_name, &message, metadata, &proto_path);
                let _ = tx.send(result);
            });
            if let Ok(result) = rx.recv_timeout(std::time::Duration::from_secs(60)) {
                match result {
                    Ok((body, elapsed)) => {
                        let body_size = body.len();
                        let _ = cx.update(|cx| {
                            response_panel.update(cx, |panel, cx| {
                                panel.set_response(ResponseData {
                                    status: 200, status_text: "OK".to_string(),
                                    headers: vec![("content-type".to_string(), "application/grpc+json".to_string()), ("grpc-status".to_string(), "0".to_string())],
                                    body, time: elapsed, size: body_size,
                                }, cx);
                            });
                        });
                    }
                    Err(e) => {
                        log::error!("gRPC error: {}", e);
                        let _ = cx.update(|cx| {
                            response_panel.update(cx, |panel, cx| {
                                panel.set_response(ResponseData { status: 0, status_text: "Error".to_string(), headers: vec![], body: format!("gRPC Error: {}", e), time: std::time::Duration::ZERO, size: 0 }, cx);
                            });
                        });
                    }
                }
                let _ = cx.update(|cx| { let _ = this.update(cx, |p, cx| { p.loading = false; cx.notify(); }); });
            }
        }).detach();
    }

    fn spawn_grpc_server_streaming(
        &mut self,
        url: String, method: GrpcMethodInfo, message: String,
        metadata: Vec<(String, String)>, proto_path: std::path::PathBuf,
        response_panel: gpui::Entity<ResponsePanel>,
        cx: &mut Context<Self>,
    ) {
        cx.spawn(async move |this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
            let result = protide_core::protocols::grpc::execute_server_streaming(&url, &method.full_name, &message, metadata, &proto_path).await;
            match result {
                Ok(chunks) => {
                    let body = chunks.join("\n---\n");
                    let body_size = body.len();
                    let _ = cx.update(|cx| {
                        response_panel.update(cx, |panel, cx| {
                            panel.set_response(ResponseData {
                                status: 200, status_text: "OK (streaming)".to_string(),
                                headers: vec![("content-type".to_string(), "application/grpc+json".to_string()), ("grpc-status".to_string(), "0".to_string()), ("x-streaming".to_string(), "true".to_string())],
                                body, time: std::time::Duration::from_secs(1), size: body_size,
                            }, cx);
                        });
                    });
                }
                Err(e) => {
                    log::error!("gRPC streaming error: {}", e);
                    let _ = cx.update(|cx| {
                        response_panel.update(cx, |panel, cx| {
                            panel.set_response(ResponseData { status: 0, status_text: "Error".to_string(), headers: vec![], body: format!("gRPC Streaming Error: {}", e), time: std::time::Duration::ZERO, size: 0 }, cx);
                        });
                    });
                }
            }
            let _ = cx.update(|cx| { let _ = this.update(cx, |p, cx| { p.loading = false; cx.notify(); }); });
        }).detach();
    }

    fn spawn_grpc_client_streaming(
        &mut self,
        url: String, method: GrpcMethodInfo, message: String,
        metadata: Vec<(String, String)>, proto_path: std::path::PathBuf,
        response_panel: gpui::Entity<ResponsePanel>,
        cx: &mut Context<Self>,
    ) {
        cx.spawn(async move |this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
            let result = protide_core::protocols::grpc::execute_client_streaming(&url, &method.full_name, vec![message], metadata, &proto_path).await;
            match result {
                Ok(body) => {
                    let body_size = body.len();
                    let _ = cx.update(|cx| {
                        response_panel.update(cx, |panel, cx| {
                            panel.set_response(ResponseData {
                                status: 200, status_text: "OK".to_string(),
                                headers: vec![("content-type".to_string(), "application/grpc+json".to_string()), ("grpc-status".to_string(), "0".to_string())],
                                body, time: std::time::Duration::from_secs(1), size: body_size,
                            }, cx);
                        });
                    });
                }
                Err(e) => {
                    log::error!("gRPC client-streaming error: {}", e);
                    let _ = cx.update(|cx| {
                        response_panel.update(cx, |panel, cx| {
                            panel.set_response(ResponseData { status: 0, status_text: "Error".to_string(), headers: vec![], body: format!("gRPC Streaming Error: {}", e), time: std::time::Duration::ZERO, size: 0 }, cx);
                        });
                    });
                }
            }
            let _ = cx.update(|cx| { let _ = this.update(cx, |p, cx| { p.loading = false; cx.notify(); }); });
        }).detach();
    }

    fn spawn_grpc_bidi(
        &mut self,
        url: String, method: GrpcMethodInfo, message: String,
        metadata: Vec<(String, String)>, proto_path: std::path::PathBuf,
        response_panel: gpui::Entity<ResponsePanel>,
        cx: &mut Context<Self>,
    ) {
        cx.spawn(async move |this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
            let result = protide_core::protocols::grpc::execute_bidi_streaming(&url, &method.full_name, vec![message], metadata, &proto_path).await;
            match result {
                Ok(chunks) => {
                    let body = chunks.join("\n---\n");
                    let body_size = body.len();
                    let _ = cx.update(|cx| {
                        response_panel.update(cx, |panel, cx| {
                            panel.set_response(ResponseData {
                                status: 200, status_text: "OK (bidi)".to_string(),
                                headers: vec![("content-type".to_string(), "application/grpc+json".to_string()), ("grpc-status".to_string(), "0".to_string()), ("x-streaming".to_string(), "true".to_string())],
                                body, time: std::time::Duration::from_secs(1), size: body_size,
                            }, cx);
                        });
                    });
                }
                Err(e) => {
                    log::error!("gRPC bidi-streaming error: {}", e);
                    let _ = cx.update(|cx| {
                        response_panel.update(cx, |panel, cx| {
                            panel.set_response(ResponseData { status: 0, status_text: "Error".to_string(), headers: vec![], body: format!("gRPC Bidi Error: {}", e), time: std::time::Duration::ZERO, size: 0 }, cx);
                        });
                    });
                }
            }
            let _ = cx.update(|cx| { let _ = this.update(cx, |p, cx| { p.loading = false; cx.notify(); }); });
        }).detach();
    }
}
