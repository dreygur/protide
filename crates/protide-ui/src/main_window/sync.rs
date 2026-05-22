use gpui::{Context, Window};
use super::*;

impl MainWindow {
    pub(super) fn connect_peer(&mut self, cx: &mut Context<Self>) {
        let code = self.join_input.read(cx).value().to_string().trim().to_string();
        if code.is_empty() {
            return;
        }
        self.presence.connection_status = ConnectionStatus::Handshaking;
        self.handshake_started = Some(std::time::Instant::now());
        cx.notify();

        if let Some(ref mut engine) = self.sync_engine {
            match engine.initiate_handshake(&code) {
                Ok(()) => {
                    self.console_panel.update(cx, |panel, cx| {
                        panel.log(ConsoleEntry::team(format!("Handshaking with code: {}", code)), cx);
                    });
                }
                Err(e) => {
                    self.console_panel.update(cx, |panel, cx| {
                        panel.log(ConsoleEntry::team(format!("Handshake error: {}", e)), cx);
                    });
                    self.presence.reset_connection();
                    cx.notify();
                }
            }
        }
    }

    pub(super) fn paste_and_join(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(item) = cx.read_from_clipboard() {
            let text = item.text().unwrap_or_default().to_string();
            self.join_input.update(cx, |input, cx| {
                input.set_value(text.clone(), window, cx);
            });
            self.connect_peer(cx);
        }
    }

    pub(super) fn poll_sync_events(&mut self, cx: &mut Context<Self>) {
        let Some(ref mut engine) = self.sync_engine else { return };

        let events = engine.tick();
        let channel_events = engine.drain_events();
        let all_events: Vec<SyncEvent> = events.into_iter().chain(channel_events).collect();

        let mut changed = false;
        let mut should_refresh_collections = false;

        for evt in all_events {
            match evt {
                SyncEvent::PeerJoined(peer_id) => {
                    let display_name = format!("Peer-{}", &peer_id[..peer_id.len().min(8)]);
                    self.presence.upsert_peer(peer_id.clone(), display_name, PeerSource::P2P);
                    self.console_panel.update(cx, |panel, cx| {
                        panel.log(
                            ConsoleEntry::team(format!("Peer joined: {}", &peer_id[..peer_id.len().min(8)])),
                            cx,
                        );
                    });
                    changed = true;
                }
                SyncEvent::PeerLeft(peer_id) => {
                    self.presence.remove_peer(&peer_id);
                    self.console_panel.update(cx, |panel, cx| {
                        panel.log(
                            ConsoleEntry::team(format!("Peer left: {}", &peer_id[..peer_id.len().min(8)])),
                            cx,
                        );
                    });
                    changed = true;
                }
                SyncEvent::LiveActivity(activity) => {
                    self.presence.upsert_peer(
                        activity.node_id.clone(),
                        activity.node_name.clone(),
                        PeerSource::UDPBroadcast,
                    );
                    let method = activity.method.clone();
                    let url = activity.url.clone();
                    let status = activity.status;
                    let time_ms = activity.time_ms;
                    self.console_panel.update(cx, |panel, cx| {
                        let msg = if status > 0 {
                            format!("{} {} → {} ({}ms)", method, url, status, time_ms)
                        } else {
                            format!("{} {}", method, url)
                        };
                        panel.log(ConsoleEntry::team(format!("[{}] {}", activity.node_name, msg)), cx);
                    });
                    changed = true;
                }
                SyncEvent::BackendStatus { backend, ready } => {
                    let status = if ready { "online" } else { "offline" };
                    self.console_panel.update(cx, |panel, cx| {
                        panel.log(ConsoleEntry::team(format!("{:?} sync {}", backend, status)), cx);
                    });
                }
                SyncEvent::SyncError(err) => {
                    self.console_panel.update(cx, |panel, cx| {
                        panel.log(ConsoleEntry::team(format!("Sync error: {}", err)), cx);
                    });
                }
                SyncEvent::EntryReceived(entry) => {
                    use protide_core::sync::DataType;
                    if entry.data_type == DataType::Collection || entry.data_type == DataType::Request {
                        should_refresh_collections = true;
                    }
                    self.console_panel.update(cx, |panel, cx| {
                        panel.log(
                            ConsoleEntry::team(format!("[sync] entry received: {:?}", entry.data_type)),
                            cx,
                        );
                    });
                    changed = true;
                }
                SyncEvent::HandshakeComplete { peer_id, peer_name } => {
                    self.handshake_started = None;
                    self.presence.set_connected(peer_id.clone(), peer_name.clone());
                    self.console_panel.update(cx, |panel, cx| {
                        panel.log(ConsoleEntry::team(format!("Connected to {}", peer_name)), cx);
                    });
                    changed = true;
                }
                SyncEvent::HandshakeFailed { reason } => {
                    self.handshake_started = None;
                    self.presence.connection_status = ConnectionStatus::Error(reason.clone());
                    self.presence.handshake_tick = false;
                    self.console_panel.update(cx, |panel, cx| {
                        panel.log(ConsoleEntry::system(format!("[PAKE] Handshake failed: {}", reason)), cx);
                    });
                    changed = true;
                }
                SyncEvent::P2PDiagnostic(msg) => {
                    self.console_panel.update(cx, |panel, cx| {
                        panel.log(ConsoleEntry::system(msg), cx);
                    });
                }
                SyncEvent::LocalAddr(addr) => {
                    self.console_panel.update(cx, |panel, cx| {
                        panel.log(ConsoleEntry::system(format!("[P2P] Listening on: {}", addr)), cx);
                    });
                }
                SyncEvent::FileReceived { relative_path, content, deleted } => {
                    let workspace = self.explorer.read(cx).workspace_path().cloned();
                    if let Some(workspace) = workspace {
                        let full_path = workspace.join(&relative_path);
                        self.explorer.update(cx, |exp, _| {
                            exp.sync_skip_paths.insert(full_path.clone());
                        });
                        if deleted {
                            let _ = std::fs::remove_file(&full_path);
                        } else {
                            if let Some(parent) = full_path.parent() {
                                let _ = std::fs::create_dir_all(parent);
                            }
                            let _ = std::fs::write(&full_path, &content);
                        }
                        self.console_panel.update(cx, |panel, cx| {
                            let action = if deleted { "deleted" } else { "synced" };
                            panel.log(ConsoleEntry::team(format!("[sync] {} {}", action, relative_path)), cx);
                        });
                        should_refresh_collections = true;
                        changed = true;
                    }
                }
            }
        }

        if should_refresh_collections {
            self.explorer.update(cx, |exp, cx| exp.refresh_collections(cx));
        }

        if self.presence.connection_status == ConnectionStatus::Handshaking {
            if let Some(started) = self.handshake_started
                && started.elapsed() > std::time::Duration::from_secs(10) {
                    self.handshake_started = None;
                    self.presence.connection_status =
                        ConnectionStatus::Error("Peer Not Found".to_string());
                    self.presence.handshake_tick = false;
                    self.console_panel.update(cx, |panel, cx| {
                        panel.log(
                            ConsoleEntry::system("[PAKE] Handshake timed out: Peer Not Found"),
                            cx,
                        );
                    });
                }
            self.presence.tick_handshake();
            changed = true;
        }

        let count_before = self.presence.peer_count();
        self.presence.reap_stale();
        if self.presence.peer_count() != count_before {
            changed = true;
        }

        if changed {
            let now = std::time::Instant::now();
            if now.duration_since(self.last_p2p_notify).as_millis() >= 100 {
                self.last_p2p_notify = now;
                cx.notify();
            }
        }
    }
}
