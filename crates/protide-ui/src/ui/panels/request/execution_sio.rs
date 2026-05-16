use gpui::Context;
use super::*;

impl<E: WebSocketExecutor> RequestPanel<E> {
    pub(super) fn connect_socketio(&mut self, cx: &mut Context<Self>) {
        if self.sio_state != SioConnectionState::Disconnected { return; }
        self.sio_state = SioConnectionState::Connecting;
        self.sio_messages.clear();
        cx.notify();

        let env_state = self.explorer_panel.as_ref().map(|p| p.read(cx).env_state().clone());
        let substitute = |s: &str| -> String {
            env_state.as_ref().map_or_else(|| s.to_string(), |e| e.substitute(s))
        };

        let headers: Vec<(String, String)> = self.headers.iter()
            .filter(|h| h.enabled && !h.key.is_empty())
            .map(|h| (substitute(&h.key), substitute(&h.value)))
            .collect();

        let sio_url = substitute(&self.url);
        log::info!("SIO connecting: {}", sio_url);
        let handle = TungsteniteSocketIoExecutor::connect(SioConnectionParams {
            url: sio_url,
            namespace: self.sio_namespace.clone(),
            headers,
        });
        self.sio_send_tx = Some(handle.cmd_tx);

        let event_rx = handle.event_rx;
        let (fwd_tx, fwd_rx) = async_channel::unbounded::<SioUiEvent>();
        std::thread::spawn(move || {
            while let Ok(ev) = event_rx.recv() {
                if fwd_tx.try_send(ev).is_err() { break; }
            }
        });
        cx.spawn(async move |this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
            while let Ok(event) = fwd_rx.recv().await {
                match event {
                    SioUiEvent::Connected { .. } => {
                        let _ = cx.update(|cx| {
                            let _ = this.update(cx, |this, cx| {
                                this.sio_state = SioConnectionState::Connected;
                                cx.notify();
                            });
                        });
                    }
                    SioUiEvent::Event(event) => {
                        let _ = cx.update(|cx| {
                            let _ = this.update(cx, |this, cx| {
                                this.sio_messages.push(event);
                                cx.notify();
                            });
                        });
                    }
                    SioUiEvent::Disconnected => {
                        let _ = cx.update(|cx| {
                            let _ = this.update(cx, |this, cx| {
                                this.sio_state = SioConnectionState::Disconnected;
                                this.sio_send_tx = None;
                                cx.notify();
                            });
                        });
                        break;
                    }
                    SioUiEvent::Error(e) => {
                        log::error!("SIO error: {}", e);
                        let _ = cx.update(|cx| {
                            let _ = this.update(cx, |this, cx| {
                                this.sio_state = SioConnectionState::Disconnected;
                                this.sio_send_tx = None;
                                this.sio_messages.push(protide_core::execution::sio::SioEvent {
                                    direction: protide_core::execution::sio::SioDirection::Received,
                                    namespace: "/".into(),
                                    event_name: "error".into(),
                                    payload: format!("\"{}\"", e),
                                    ack_id: None,
                                    is_ack: false,
                                    timestamp: chrono::Local::now(),
                                });
                                cx.notify();
                            });
                        });
                        break;
                    }
                }
            }
            let _ = cx.update(|cx| {
                let _ = this.update(cx, |this, cx| {
                    if this.sio_state != SioConnectionState::Disconnected {
                        this.sio_state = SioConnectionState::Disconnected;
                        this.sio_send_tx = None;
                        cx.notify();
                    }
                });
            });
        }).detach();
    }

    pub(super) fn disconnect_socketio(&mut self, cx: &mut Context<Self>) {
        if let Some(tx) = self.sio_send_tx.take() {
            let _ = tx.send(SioCommand::Disconnect);
        }
        self.sio_state = SioConnectionState::Disconnected;
        cx.notify();
    }

    pub(super) fn emit_socketio_event(&mut self, cx: &mut Context<Self>) {
        if self.sio_state != SioConnectionState::Connected { return; }
        let env_state = self.explorer_panel.as_ref().map(|p| p.read(cx).env_state().clone());
        let substitute = |s: &str| -> String {
            env_state.as_ref().map_or_else(|| s.to_string(), |e| e.substitute(s))
        };
        let payload = substitute(&self.sio_payload_editor.read(cx).content().to_string());
        let namespace = substitute(&self.sio_namespace);
        let event_name = substitute(&self.sio_event_name);
        let ack_id = if self.sio_want_ack {
            let id = self.sio_next_ack_id;
            self.sio_next_ack_id = self.sio_next_ack_id.wrapping_add(1);
            Some(id)
        } else {
            None
        };
        if let Some(tx) = &self.sio_send_tx {
            let _ = tx.send(SioCommand::Emit {
                namespace,
                event_name,
                payload,
                ack_id,
            });
            cx.notify();
        }
    }
}
