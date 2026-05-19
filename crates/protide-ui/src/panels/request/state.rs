use gpui::{Context, Window};
use super::*;

impl<E: WebSocketExecutor> RequestPanel<E> {
    /// Set the explorer panel reference for environment variable substitution
    pub fn set_explorer_panel(&mut self, explorer_panel: Entity<ExplorerPanel>, cx: &mut Context<Self>) {
        self.explorer_panel = Some(explorer_panel);
        cx.notify();
    }

    /// Connect the shared console panel so every request is logged.
    pub fn set_console_panel(&mut self, console: Entity<ConsolePanel>, cx: &mut Context<Self>) {
        self.console_panel = Some(console);
        cx.notify();
    }

    pub fn has_response_panel(&self) -> bool {
        !matches!(self.request_mode, RequestMode::WebSocket | RequestMode::SocketIo)
    }

    /// Get the current request mode label for status bar display
    pub fn mode_label(&self) -> &'static str {
        match self.request_mode {
            RequestMode::Http => "HTTP",
            RequestMode::GraphQL => "GraphQL",
            RequestMode::WebSocket => "WebSocket",
            RequestMode::Grpc => "gRPC",
            RequestMode::Trpc => "tRPC",
            RequestMode::SocketIo => "Socket.IO",
        }
    }

    /// Set request mode (HTTP, GraphQL, or WebSocket)
    pub(super) fn set_request_mode(&mut self, mode: RequestMode, cx: &mut Context<Self>) {
        if self.request_mode == mode {
            return;
        }
        self.request_mode = mode;
        self.active_tab = 0;
        self.active_edit = None;
        self.edit_selection = 0..0;
        match mode {
            RequestMode::GraphQL => {
                self.method = HttpMethod::Post;
            }
            RequestMode::WebSocket => {
                // WebSocket uses ws:// or wss:// URL
                if !self.url.starts_with("ws://") && !self.url.starts_with("wss://") {
                    self.url = "wss://echo.websocket.org".to_string();
                    let len = self.url.chars().count();
                    self.url_selection = len..len;
                }
            }
            RequestMode::Grpc => {
                // gRPC uses grpc:// URL scheme
                if !self.url.contains("grpc") {
                    self.url = "grpc://localhost:50051".to_string();
                    let len = self.url.chars().count();
                    self.url_selection = len..len;
                }
            }
            RequestMode::Trpc => {
                self.method = HttpMethod::Post;
                if !self.url.ends_with("/trpc") {
                    self.url = "http://localhost:3000/trpc".to_string();
                    let len = self.url.chars().count();
                    self.url_selection = len..len;
                }
            }
            RequestMode::SocketIo => {
                if !self.url.starts_with("http://") && !self.url.starts_with("https://") {
                    self.url = "http://localhost:3000".to_string();
                    let len = self.url.chars().count();
                    self.url_selection = len..len;
                }
            }
            RequestMode::Http => {}
        }
        cx.notify();
    }

    pub(super) fn set_tab(&mut self, index: usize, cx: &mut Context<Self>) {
        self.active_tab = index;
        self.active_edit = None;
        self.edit_selection = 0..0;
        cx.notify();
    }

    pub(super) fn toggle_method_dropdown(&mut self, cx: &mut Context<Self>) {
        self.method_dropdown_open = !self.method_dropdown_open;
        cx.notify();
    }

    pub(super) fn select_method(&mut self, method: HttpMethod, cx: &mut Context<Self>) {
        self.method = method;
        self.method_dropdown_open = false;
        cx.notify();
    }

    pub(super) fn set_auth_type(&mut self, auth_type: AuthType, cx: &mut Context<Self>) {
        self.auth_type = auth_type;
        self.active_edit = None;
        cx.notify();
    }

    pub(super) fn toggle_api_key_location(&mut self, cx: &mut Context<Self>) {
        self.api_key_location = match self.api_key_location {
            ApiKeyLocation::Header => ApiKeyLocation::QueryParam,
            ApiKeyLocation::QueryParam => ApiKeyLocation::Header,
        };
        cx.notify();
    }

    pub(super) fn focus_url(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.url_focus.focus(window, cx);
    }
}
