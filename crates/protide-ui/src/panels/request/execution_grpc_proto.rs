use super::*;
use gpui::Context;

impl<E: WebSocketExecutor> RequestPanel<E> {
    pub(super) fn load_proto_file(&mut self, cx: &mut Context<Self>) {
        file_dialog::prompt(
            cx,
            Pick::File,
            |d| {
                let d = d
                    .add_filter("Proto Files", &["proto"])
                    .set_title("Select Proto File");
                match last_paths::last_dir("proto_file") {
                    Some(dir) => d.set_directory(dir),
                    None => d,
                }
            },
            |this, path, cx| {
                last_paths::save_last_dir("proto_file", &path);
                this.load_grpc_proto_from_path(path, cx);
            },
        );
    }

    pub fn load_grpc_proto_from_path(&mut self, path: std::path::PathBuf, cx: &mut Context<Self>) {
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                self.grpc_proto_path = Some(path);
                self.grpc_proto_content = content.clone();
                self.parse_proto_services(&content);
                log::info!(
                    "Proto loaded: {} ({} services)",
                    self.grpc_proto_path.as_ref().unwrap().display(),
                    self.grpc_services.len()
                );
                cx.notify();
            }
            Err(e) => {
                log::error!("Failed to read proto file: {}", e);
            }
        }
    }

    pub(super) fn parse_proto_services(&mut self, content: &str) {
        self.grpc_services.clear();
        self.grpc_methods.clear();
        self.grpc_service = None;
        self.grpc_method = None;

        if let Some(ref path) = self.grpc_proto_path.clone()
            && let Ok(pool) = protide_core::protocols::grpc::parse_proto_file(path)
        {
            for svc in pool.services() {
                let svc_name = svc.full_name().to_string();
                self.grpc_services.push(svc_name.clone());
                for method in svc.methods() {
                    let streaming_type =
                        match (method.is_client_streaming(), method.is_server_streaming()) {
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
            if let Some(s) = self.grpc_services.first() {
                self.grpc_service = Some(s.clone());
            }
            if let Some(m) = self.grpc_methods.first() {
                self.grpc_method = Some(m.clone());
            }
            return;
        }

        // Fallback: basic text parsing
        let mut in_service = false;
        let mut current_service = String::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("service ")
                && let Some(name) = trimmed
                    .strip_prefix("service ")
                    .and_then(|s| s.split_whitespace().next())
            {
                current_service = name.to_string();
                self.grpc_services.push(current_service.clone());
                in_service = true;
            }
            if in_service
                && trimmed.starts_with("rpc ")
                && let Some(name) = trimmed
                    .strip_prefix("rpc ")
                    .and_then(|s| s.split('(').next())
                    .map(|s| s.trim())
            {
                self.grpc_methods.push(GrpcMethodInfo {
                    full_name: format!("{}/{}", current_service, name),
                    streaming_type: GrpcStreamingType::Unary,
                });
            }
            if in_service && trimmed == "}" {
                in_service = false;
            }
        }
        if let Some(s) = self.grpc_services.first() {
            self.grpc_service = Some(s.clone());
        }
        if let Some(m) = self.grpc_methods.first() {
            self.grpc_method = Some(m.clone());
        }
    }
}
