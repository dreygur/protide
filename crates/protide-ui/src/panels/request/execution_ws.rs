use gpui::Context;
use super::*;
use super::graphql::dns_troubleshoot_hint;

impl<E: WebSocketExecutor> RequestPanel<E> {
    /// Connect to WebSocket server
    pub(super) fn connect_websocket(&mut self, cx: &mut Context<Self>) {
        if !matches!(self.ws_state, WsConnectionState::Disconnected | WsConnectionState::Error) {
            return;
        }

        self.ws_state = WsConnectionState::Connecting;
        self.ws_messages.clear();
        cx.notify();

        let env_state = self.explorer_panel.as_ref().map(|p| p.read(cx).env_state().clone());
        let substitute = |s: &str| -> String {
            env_state.as_ref().map_or_else(|| s.to_string(), |e| e.substitute(s))
        };

        let url = substitute(&self.url);
        let headers: Vec<(String, String)> = self.headers.iter()
            .filter(|h| h.enabled && !h.key.is_empty())
            .map(|h| (substitute(&h.key), substitute(&h.value)))
            .collect();
        let on_message_script = self.pre_script_editor.read(cx).content().to_string();
        let env_vars: std::collections::HashMap<String, String> = env_state.as_ref()
            .and_then(|e| e.active())
            .map(|env| env.variables.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default();
        let explorer_panel = self.explorer_panel.clone();
        let ws_console_panel = self.console_panel.clone();
        let ws_log_url = url.clone();
        log::info!("WS connecting: {}", ws_log_url);
        let ws_protocol = match self.request_mode {
            RequestMode::SocketIo => "Socket.IO",
            _ => "WebSocket",
        }.to_string();

        let handle = E::connect(WsConnectionParams { url, headers, on_message_script, env_vars });
        self.ws_send_tx = Some(handle.cmd_tx);

        let event_rx = handle.event_rx;
        let (fwd_tx, fwd_rx) = async_channel::unbounded::<WsEvent>();
        std::thread::spawn(move || {
            while let Ok(ev) = event_rx.recv() {
                if fwd_tx.try_send(ev).is_err() { break; }
            }
        });
        cx.spawn(async move |this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
            while let Ok(event) = fwd_rx.recv().await {
                match event {
                    WsEvent::Connected => {
                        log::info!("WS connected: {}", ws_log_url);
                        let _ = cx.update(|cx| {
                            let _ = this.update(cx, |this, cx| {
                                this.ws_state = WsConnectionState::Connected;
                                cx.notify();
                            });
                        });
                    }
                    WsEvent::Message { msg, env_changes } => {
                        let _ = cx.update(|cx| {
                            let _ = this.update(cx, |this, cx| {
                                for (k, v) in &env_changes {
                                    if let Some(ref ep) = explorer_panel {
                                        ep.update(cx, |p, cx| p.set_env_variable(k, v, cx));
                                    }
                                }
                                this.ws_messages.push(msg);
                                this.ws_scroll.scroll_to_bottom();
                                cx.notify();
                            });
                        });
                    }
                    WsEvent::Disconnected => {
                        log::info!("WS disconnected: {}", ws_log_url);
                        let _ = cx.update(|cx| {
                            let _ = this.update(cx, |this, cx| {
                                this.ws_state = WsConnectionState::Disconnected;
                                this.ws_send_tx = None;
                                cx.notify();
                            });
                        });
                        break;
                    }
                    WsEvent::Error(e) => {
                        log::error!("WS error {}: {}", ws_log_url, e);
                        let _ = cx.update(|cx| {
                            let hint = dns_troubleshoot_hint(&e);
                            if let Some(ref console) = ws_console_panel {
                                let entry = ConsoleEntry {
                                    timestamp: chrono::Local::now(),
                                    level: LogLevel::Error,
                                    source: ConsoleEntrySource::Request,
                                    protocol: ws_protocol.clone(),
                                    method: "CONNECT".to_string(),
                                    url: ws_log_url.clone(),
                                    status: 0,
                                    duration_ms: 0,
                                    error: Some(e.clone()),
                                    response_body: String::new(),
                                    troubleshoot_hint: hint,
                                };
                                console.update(cx, |panel, cx| panel.log(entry, cx));
                            }
                            let _ = this.update(cx, |this, cx| {
                                this.ws_state = WsConnectionState::Error;
                                this.ws_send_tx = None;
                                this.ws_messages.push(WsMessage {
                                    direction: WsDirection::Received,
                                    content: format!("Connection failed: {}", e),
                                    timestamp: chrono::Local::now(),
                                });
                                this.ws_scroll.scroll_to_bottom();
                                cx.notify();
                            });
                        });
                        break;
                    }
                }
            }
            let _ = cx.update(|cx| {
                let _ = this.update(cx, |this, cx| {
                    if !matches!(this.ws_state, WsConnectionState::Disconnected | WsConnectionState::Error) {
                        this.ws_state = WsConnectionState::Disconnected;
                        this.ws_send_tx = None;
                        cx.notify();
                    }
                });
            });
        }).detach();
    }

    pub(super) fn disconnect_websocket(&mut self, cx: &mut Context<Self>) {
        if let Some(tx) = self.ws_send_tx.take() {
            let _ = tx.send(WsCommand::Disconnect);
        }
        self.ws_state = WsConnectionState::Disconnected;
        cx.notify();
    }

    pub(super) fn send_websocket_message(&mut self, cx: &mut Context<Self>) {
        if self.ws_state != WsConnectionState::Connected { return; }
        let message = self.ws_message_editor.read(cx).content();
        if message.trim().is_empty() { return; }
        if let Some(tx) = &self.ws_send_tx {
            let _ = tx.send(WsCommand::Send(message.to_string()));
            cx.notify();
        }
    }
}
