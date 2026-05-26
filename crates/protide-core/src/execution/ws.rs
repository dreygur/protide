use std::collections::{HashMap, VecDeque};
use std::sync::mpsc;

use crate::scripting::context::{ResponseData as ScriptResponseData, ScriptContext};
use crate::scripting::ScriptEngine;

// ── Public types ─────────────────────────────────────────────────────────────

/// Direction of a WebSocket message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WsDirection {
    Sent,
    Received,
}

/// A single WebSocket message - stored in the ring buffer.
#[derive(Debug, Clone)]
pub struct WsMessage {
    pub direction: WsDirection,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Local>,
}

/// Events emitted by the executor to the UI event loop.
#[derive(Debug)]
pub enum WsEvent {
    Connected,
    Message {
        msg: WsMessage,
        /// Env var changes produced by the on-message script (may be empty).
        env_changes: Vec<(String, String)>,
    },
    Disconnected,
    Error(String),
}

/// Commands sent from the UI into the running session.
pub enum WsCommand {
    Send(String),
    Disconnect,
}

/// Thin session handle returned by `WebSocketExecutor::connect`.
/// The UI owns this; dropping `cmd_tx` is equivalent to sending `Disconnect`.
pub struct WsHandle {
    pub cmd_tx: mpsc::Sender<WsCommand>,
    pub event_rx: mpsc::Receiver<WsEvent>,
}

/// Parameters for opening a WebSocket connection.
pub struct WsConnectionParams {
    pub url: String,
    pub headers: Vec<(String, String)>,
    /// Optional JS script run on every received message.
    /// Context: `response.body` = message content; `env.set()` / `env.get()` work normally.
    pub on_message_script: String,
    pub env_vars: HashMap<String, String>,
}

// ── Ring buffer ───────────────────────────────────────────────────────────────

/// Bounded ring buffer for WebSocket message history.
/// Oldest messages are evicted when `cap` is exceeded, preventing unbounded growth.
#[derive(Clone)]
pub struct WsRingBuffer {
    buf: VecDeque<WsMessage>,
    cap: usize,
}

impl WsRingBuffer {
    pub fn new(cap: usize) -> Self {
        Self { buf: VecDeque::with_capacity(cap), cap }
    }

    pub fn push(&mut self, msg: WsMessage) {
        if self.buf.len() >= self.cap {
            self.buf.pop_front();
        }
        self.buf.push_back(msg);
    }

    pub fn clear(&mut self) {
        self.buf.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &WsMessage> {
        self.buf.iter()
    }

    /// Maximum messages this buffer will hold.
    pub fn capacity(&self) -> usize {
        self.cap
    }
}

impl Default for WsRingBuffer {
    fn default() -> Self {
        Self::new(1_000)
    }
}

// ── Executor trait ────────────────────────────────────────────────────────────

/// Abstracts over WebSocket backends - enables mock executors in tests.
///
/// The `'static` supertrait is required so `RequestPanel<E>` can be used as a
/// GPUI entity (which demands `'static` on its type parameter).
pub trait WebSocketExecutor: 'static {
    /// Open a connection. Returns immediately; the session runs in a background thread.
    fn connect(params: WsConnectionParams) -> WsHandle
    where
        Self: Sized;
}

// ── Production executor ───────────────────────────────────────────────────────

/// Production WebSocket executor backed by `tokio-tungstenite`.
pub struct TungsteniteExecutor;

impl WebSocketExecutor for TungsteniteExecutor {
    fn connect(params: WsConnectionParams) -> WsHandle {
        let (cmd_tx, cmd_rx) = mpsc::channel::<WsCommand>();
        let (event_tx, event_rx) = mpsc::channel::<WsEvent>();

        std::thread::spawn(move || {
            let rt = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    let _ = event_tx.send(WsEvent::Error(
                        format!("Failed to start WebSocket runtime: {}", e),
                    ));
                    return;
                }
            };
            rt.block_on(run_connection(params, cmd_rx, event_tx));
        });

        WsHandle { cmd_tx, event_rx }
    }
}

// ── Connection task ───────────────────────────────────────────────────────────

async fn run_connection(
    params: WsConnectionParams,
    cmd_rx: mpsc::Receiver<WsCommand>,
    event_tx: mpsc::Sender<WsEvent>,
) {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::connect_async;

    let conn = connect_async(&params.url).await;
    match conn {
        Err(e) => {
            let msg = friendly_ws_error(&e);
            let _ = event_tx.send(WsEvent::Error(msg));
        }
        Ok((ws_stream, _)) => {
            let _ = event_tx.send(WsEvent::Connected);
            let (mut write, mut read) = ws_stream.split();

            // Send a ping every 30 s to prevent idle-connection drops by routers/NAT.
            const PING_INTERVAL: std::time::Duration = std::time::Duration::from_secs(30);
            let mut last_ping = std::time::Instant::now();

            loop {
                // ── Keepalive ping ────────────────────────────────────────
                if last_ping.elapsed() >= PING_INTERVAL {
                    let ping = tokio_tungstenite::tungstenite::Message::Ping(vec![].into());
                    if write.send(ping).await.is_err() {
                        break;
                    }
                    last_ping = std::time::Instant::now();
                }

                // ── Outgoing ──────────────────────────────────────────────
                match cmd_rx.try_recv() {
                    Ok(WsCommand::Send(text)) => {
                        let frame =
                            tokio_tungstenite::tungstenite::Message::Text(text.clone().into());
                        if write.send(frame).await.is_err() {
                            break;
                        }
                        let _ = event_tx.send(WsEvent::Message {
                            msg: WsMessage {
                                direction: WsDirection::Sent,
                                content: text,
                                timestamp: chrono::Local::now(),
                            },
                            env_changes: vec![],
                        });
                    }
                    Ok(WsCommand::Disconnect) => break,
                    Err(mpsc::TryRecvError::Disconnected) => break,
                    Err(mpsc::TryRecvError::Empty) => {}
                }

                // ── Incoming (50 ms poll) ─────────────────────────────────
                let poll = tokio::time::timeout(
                    std::time::Duration::from_millis(50),
                    read.next(),
                )
                .await;

                match poll {
                    Ok(Some(Ok(tokio_tungstenite::tungstenite::Message::Text(text)))) => {
                        let content = text.to_string();
                        let env_changes = if params.on_message_script.trim().is_empty() {
                            vec![]
                        } else {
                            run_message_script(
                                &params.on_message_script,
                                &params.env_vars,
                                &content,
                            )
                        };
                        let _ = event_tx.send(WsEvent::Message {
                            msg: WsMessage {
                                direction: WsDirection::Received,
                                content,
                                timestamp: chrono::Local::now(),
                            },
                            env_changes,
                        });
                    }
                    Ok(Some(Ok(tokio_tungstenite::tungstenite::Message::Ping(data)))) => {
                        // Auto-respond to server pings to keep connection alive
                        let pong = tokio_tungstenite::tungstenite::Message::Pong(data);
                        let _ = write.send(pong).await;
                    }
                    Ok(Some(Ok(tokio_tungstenite::tungstenite::Message::Close(_)))) | Ok(None) => {
                        break
                    }
                    Ok(Some(Err(_))) => break,
                    Ok(Some(Ok(_))) => {} // binary, pong - ignore
                    Err(_) => {}          // timeout, loop again
                }
            }

            let _ = event_tx.send(WsEvent::Disconnected);
        }
    }
}

// ── Error formatting ──────────────────────────────────────────────────────────

fn friendly_ws_error(e: &tokio_tungstenite::tungstenite::Error) -> String {
    let msg = e.to_string().to_lowercase();
    if msg.contains("failed to lookup")
        || msg.contains("no such host")
        || msg.contains("nodename nor servname")
        || msg.contains("name or service not known")
        || msg.contains("could not resolve")
    {
        "Unable to resolve host. Check your internet connection.".to_string()
    } else {
        e.to_string()
    }
}

// ── Scripting hook ────────────────────────────────────────────────────────────

/// Run the on-message script. Returns env var changes produced by `env.set()` calls.
/// Silently swallows engine errors so a broken script never kills the connection.
fn run_message_script(
    script: &str,
    env_vars: &HashMap<String, String>,
    message_content: &str,
) -> Vec<(String, String)> {
    let Ok(engine) = ScriptEngine::new() else {
        return vec![];
    };
    let resp = ScriptResponseData::new(0, "ws_message", message_content.to_string());
    let mut ctx = ScriptContext::new().with_env(env_vars.clone());
    ctx.set_response(resp);
    match engine.run_post_script(script, &mut ctx) {
        Ok(outcome) => outcome.env_changes,
        Err(_) => vec![],
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_buffer_evicts_oldest() {
        let mut rb = WsRingBuffer::new(3);
        for i in 0..5u8 {
            rb.push(WsMessage {
                direction: WsDirection::Received,
                content: i.to_string(),
                timestamp: chrono::Local::now(),
            });
        }
        assert_eq!(rb.len(), 3);
        let contents: Vec<&str> = rb.iter().map(|m| m.content.as_str()).collect();
        assert_eq!(contents, ["2", "3", "4"]);
    }

    #[test]
    fn ring_buffer_clone_is_independent() {
        let mut rb = WsRingBuffer::new(4);
        rb.push(WsMessage {
            direction: WsDirection::Sent,
            content: "hello".into(),
            timestamp: chrono::Local::now(),
        });
        let mut rb2 = rb.clone();
        rb2.clear();
        assert_eq!(rb.len(), 1);
        assert_eq!(rb2.len(), 0);
    }

    #[test]
    fn test_ws_ring_buffer_evicts_oldest() {
        let mut buf = WsRingBuffer::new(3);
        for i in 0u8..5 {
            buf.push(WsMessage {
                direction: WsDirection::Sent,
                content: i.to_string(),
                timestamp: chrono::Local::now(),
            });
        }
        assert_eq!(buf.len(), 3);
        let contents: Vec<_> = buf.iter().map(|m| m.content.clone()).collect();
        assert_eq!(contents, vec!["2", "3", "4"]);
    }

    #[test]
    fn test_ws_ring_buffer_cap_not_exceeded() {
        let mut buf = WsRingBuffer::new(5);
        for i in 0..10u8 {
            buf.push(WsMessage {
                direction: WsDirection::Received,
                content: i.to_string(),
                timestamp: chrono::Local::now(),
            });
        }
        assert_eq!(buf.len(), 5);
    }

    #[test]
    fn test_ws_ring_buffer_empty() {
        let buf = WsRingBuffer::new(10);
        assert!(buf.is_empty());
        assert_eq!(buf.len(), 0);
    }
}
