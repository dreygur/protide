use std::collections::VecDeque;
use std::sync::mpsc;

// ── Public types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SioDirection {
    Sent,
    Received,
}

/// A single Socket.IO event stored in the ring buffer.
#[derive(Debug, Clone)]
pub struct SioEvent {
    pub direction: SioDirection,
    pub namespace: String,
    pub event_name: String,
    pub payload: String,
    pub ack_id: Option<u32>,
    /// True if this entry is an ACK response rather than a regular event.
    pub is_ack: bool,
    pub timestamp: chrono::DateTime<chrono::Local>,
}

/// Events emitted by the executor to the UI event loop.
#[derive(Debug)]
pub enum SioUiEvent {
    Connected { namespace: String },
    Event(SioEvent),
    Disconnected,
    Error(String),
}

/// Commands sent from the UI into the running session.
pub enum SioCommand {
    Emit {
        namespace: String,
        event_name: String,
        payload: String,
        ack_id: Option<u32>,
    },
    Disconnect,
}

/// Thin session handle returned by `SocketIoExecutor::connect`.
pub struct SioHandle {
    pub cmd_tx: mpsc::Sender<SioCommand>,
    pub event_rx: mpsc::Receiver<SioUiEvent>,
}

/// Parameters for opening a Socket.IO connection.
pub struct SioConnectionParams {
    pub url: String,
    pub namespace: String,
    pub headers: Vec<(String, String)>,
}

// ── Ring buffer ───────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct SioRingBuffer {
    buf: VecDeque<SioEvent>,
    cap: usize,
}

impl SioRingBuffer {
    pub fn new(cap: usize) -> Self {
        Self { buf: VecDeque::with_capacity(cap), cap }
    }

    pub fn push(&mut self, event: SioEvent) {
        if self.buf.len() >= self.cap {
            self.buf.pop_front();
        }
        self.buf.push_back(event);
    }

    pub fn clear(&mut self) { self.buf.clear(); }
    pub fn is_empty(&self) -> bool { self.buf.is_empty() }
    pub fn len(&self) -> usize { self.buf.len() }
    pub fn iter(&self) -> impl Iterator<Item = &SioEvent> { self.buf.iter() }
    pub fn capacity(&self) -> usize { self.cap }
}

impl Default for SioRingBuffer {
    fn default() -> Self { Self::new(1_000) }
}

// ── Executor trait ────────────────────────────────────────────────────────────

pub trait SocketIoExecutor: 'static {
    fn connect(params: SioConnectionParams) -> SioHandle
    where
        Self: Sized;
}

// ── Production executor ───────────────────────────────────────────────────────

pub struct TungsteniteSocketIoExecutor;

impl SocketIoExecutor for TungsteniteSocketIoExecutor {
    fn connect(params: SioConnectionParams) -> SioHandle {
        let (cmd_tx, cmd_rx) = mpsc::channel::<SioCommand>();
        let (event_tx, event_rx) = mpsc::channel::<SioUiEvent>();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("sio tokio runtime");
            rt.block_on(run_connection(params, cmd_rx, event_tx));
        });

        SioHandle { cmd_tx, event_rx }
    }
}

// ── Protocol helpers ──────────────────────────────────────────────────────────

/// Convert a base URL (http/https) to the Engine.IO WebSocket endpoint.
fn build_ws_url(base_url: &str) -> String {
    let base = base_url.trim_end_matches('/');
    // Convert scheme: http → ws, https → wss
    let ws_base = if base.starts_with("https://") {
        base.replacen("https://", "wss://", 1)
    } else {
        base.replacen("http://", "ws://", 1)
    };
    // Append Socket.IO path unless already present
    if ws_base.contains("/socket.io") {
        ws_base
    } else {
        format!("{}/socket.io/?EIO=4&transport=websocket", ws_base)
    }
}

/// Encode a Socket.IO CONNECT packet for a namespace.
fn encode_sio_connect(namespace: &str) -> String {
    if namespace == "/" {
        "0".to_string()
    } else {
        format!("0{},", namespace)
    }
}

/// Encode a Socket.IO EVENT packet (type 2).
fn encode_sio_event(namespace: &str, event_name: &str, payload: &str, ack_id: Option<u32>) -> String {
    let escaped_name = event_name.replace('\\', "\\\\").replace('"', "\\\"");
    let data = format!("[\"{}\",{}]", escaped_name, payload);
    let ack_str = ack_id.map(|id| id.to_string()).unwrap_or_default();
    if namespace == "/" {
        format!("2{}{}", ack_str, data)
    } else {
        format!("2{},{}{}", namespace, ack_str, data)
    }
}

/// Return (eio_type_char, rest) for an Engine.IO raw message.
fn parse_eio_type(raw: &str) -> Option<(char, &str)> {
    let c = raw.chars().next()?;
    Some((c, &raw[c.len_utf8()..]))
}

/// Parse a Socket.IO packet. Returns (ptype, namespace, ack_id, data_slice).
fn parse_sio_header(raw: &str) -> Option<(u8, String, Option<u32>, &str)> {
    let ptype = raw.chars().next()?.to_digit(10)? as u8;
    let rest = &raw[1..];

    let (namespace, after_ns) = if rest.starts_with('/') {
        // Namespace ends at ',' or (for DISCONNECT) at end of string
        if let Some(pos) = rest.find(',') {
            (rest[..pos].to_string(), &rest[pos + 1..])
        } else {
            (rest.to_string(), "")
        }
    } else {
        ("/".to_string(), rest)
    };

    // Optional ack id: leading digits before the first non-digit (e.g. '[')
    let digit_end = after_ns.find(|c: char| !c.is_ascii_digit()).unwrap_or(after_ns.len());
    let (ack_id, data) = if digit_end > 0 && !after_ns.starts_with('[') {
        (after_ns[..digit_end].parse::<u32>().ok(), &after_ns[digit_end..])
    } else {
        (None, after_ns)
    };

    Some((ptype, namespace, ack_id, data))
}

/// Extract (event_name, payload) from a Socket.IO event JSON array.
fn parse_event_array(json: &str) -> Option<(String, String)> {
    let arr: serde_json::Value = serde_json::from_str(json).ok()?;
    let arr = arr.as_array()?;
    let name = arr.first()?.as_str()?.to_string();
    let payload = if arr.len() > 1 {
        serde_json::to_string(&arr[1]).unwrap_or_else(|_| "null".into())
    } else {
        "null".to_string()
    };
    Some((name, payload))
}

// ── Connection task ───────────────────────────────────────────────────────────

async fn run_connection(
    params: SioConnectionParams,
    cmd_rx: mpsc::Receiver<SioCommand>,
    event_tx: mpsc::Sender<SioUiEvent>,
) {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::{connect_async, tungstenite::Message};

    let ws_url = build_ws_url(&params.url);

    let (mut write, mut read) = match connect_async(&ws_url).await {
        Err(e) => {
            let _ = event_tx.send(SioUiEvent::Error(format!("Connection failed: {}", e)));
            return;
        }
        Ok((stream, _)) => stream.split(),
    };

    // ── Engine.IO OPEN ────────────────────────────────────────────────────────
    // Server sends "0{...json...}" immediately after the WebSocket upgrade.
    match tokio::time::timeout(std::time::Duration::from_secs(10), read.next()).await {
        Ok(Some(Ok(Message::Text(text)))) => match parse_eio_type(&text) {
            Some(('0', _)) => {} // OPEN — we don't need sid/pingInterval
            _ => {
                let _ = event_tx.send(SioUiEvent::Error("Unexpected EIO handshake packet".into()));
                return;
            }
        },
        _ => {
            let _ = event_tx.send(SioUiEvent::Error("EIO OPEN timeout or error".into()));
            return;
        }
    }

    // ── Socket.IO namespace CONNECT ───────────────────────────────────────────
    let connect_pkt = format!("4{}", encode_sio_connect(&params.namespace));
    if write.send(Message::Text(connect_pkt.into())).await.is_err() {
        let _ = event_tx.send(SioUiEvent::Error("Failed to send SIO CONNECT".into()));
        return;
    }

    // Await the SIO CONNECT acknowledgement (type 0 back from server).
    // Some servers send an EIO PING first; we handle that before retrying.
    if !await_sio_connect_ack(&mut write, &mut read, &params.namespace, &event_tx).await {
        return;
    }

    let _ = event_tx.send(SioUiEvent::Connected { namespace: params.namespace.clone() });

    // ── Main event loop ───────────────────────────────────────────────────────
    loop {
        // Outgoing commands
        match cmd_rx.try_recv() {
            Ok(SioCommand::Emit { namespace: ns, event_name, payload, ack_id }) => {
                let sio = encode_sio_event(&ns, &event_name, &payload, ack_id);
                let eio = format!("4{}", sio);
                if write.send(Message::Text(eio.into())).await.is_err() {
                    break;
                }
                let _ = event_tx.send(SioUiEvent::Event(SioEvent {
                    direction: SioDirection::Sent,
                    namespace: ns,
                    event_name,
                    payload,
                    ack_id,
                    is_ack: false,
                    timestamp: chrono::Local::now(),
                }));
            }
            Ok(SioCommand::Disconnect) => break,
            Err(mpsc::TryRecvError::Disconnected) => break,
            Err(mpsc::TryRecvError::Empty) => {}
        }

        // Incoming (50 ms poll, same cadence as WS)
        let poll = tokio::time::timeout(
            std::time::Duration::from_millis(50),
            read.next(),
        ).await;

        match poll {
            Ok(Some(Ok(Message::Text(text)))) => {
                match parse_eio_type(&text) {
                    Some(('2', probe)) => {
                        // EIO PING → PONG (keep-alive)
                        let pong = format!("3{}", probe);
                        if write.send(Message::Text(pong.into())).await.is_err() {
                            break;
                        }
                    }
                    Some(('4', sio)) => match parse_sio_header(sio) {
                        Some((2, ns, ack_id, data)) => {
                            if let Some((name, payload)) = parse_event_array(data) {
                                let _ = event_tx.send(SioUiEvent::Event(SioEvent {
                                    direction: SioDirection::Received,
                                    namespace: ns,
                                    event_name: name,
                                    payload,
                                    ack_id,
                                    is_ack: false,
                                    timestamp: chrono::Local::now(),
                                }));
                            }
                        }
                        Some((3, ns, ack_id, data)) => {
                            // ACK response
                            let _ = event_tx.send(SioUiEvent::Event(SioEvent {
                                direction: SioDirection::Received,
                                namespace: ns,
                                event_name: match ack_id { Some(id) => format!("ack#{}", id), None => "ack".to_string() },
                                payload: data.to_string(),
                                ack_id,
                                is_ack: true,
                                timestamp: chrono::Local::now(),
                            }));
                        }
                        Some((1, _, _, _)) => break, // SIO DISCONNECT
                        _ => {}
                    },
                    Some(('1', _)) | None => break, // EIO CLOSE or unknown
                    _ => {}
                }
            }
            Ok(Some(Ok(Message::Close(_)))) | Ok(None) => break,
            Ok(Some(Err(_))) => break,
            Ok(Some(Ok(_))) => {} // binary, ping, pong — ignore
            Err(_) => {}          // 50 ms timeout, loop again
        }
    }

    let _ = event_tx.send(SioUiEvent::Disconnected);
}

/// Wait for the SIO CONNECT ack. Returns true on success, false on error.
/// Some servers send an EIO PING before the SIO CONNECT ack — we handle one
/// such ping transparently before giving up.
async fn await_sio_connect_ack<W, R>(
    write: &mut W,
    read: &mut R,
    namespace: &str,
    event_tx: &mpsc::Sender<SioUiEvent>,
) -> bool
where
    W: futures_util::Sink<tokio_tungstenite::tungstenite::Message, Error = tokio_tungstenite::tungstenite::Error>
        + Unpin,
    R: futures_util::Stream<
            Item = Result<
                tokio_tungstenite::tungstenite::Message,
                tokio_tungstenite::tungstenite::Error,
            >,
        > + Unpin,
{
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::Message;

    let mut pings_handled = 0u8;
    loop {
        match tokio::time::timeout(std::time::Duration::from_secs(10), read.next()).await {
            Ok(Some(Ok(Message::Text(text)))) => match parse_eio_type(&text) {
                Some(('4', sio)) => match parse_sio_header(sio) {
                    Some((0, _, _, _)) => return true,
                    Some((4, _, _, data)) => {
                        let msg = serde_json::from_str::<serde_json::Value>(data)
                            .ok()
                            .and_then(|v| {
                                v.get("message").and_then(|m| m.as_str()).map(String::from)
                            })
                            .unwrap_or_else(|| data.to_string());
                        let _ = event_tx.send(SioUiEvent::Error(format!("Connect error: {}", msg)));
                        return false;
                    }
                    _ => {
                        let _ = event_tx.send(SioUiEvent::Error("Unexpected SIO packet during connect".into()));
                        return false;
                    }
                },
                // Some servers send EIO PINGs before the SIO CONNECT ack — reply and keep waiting
                Some(('2', probe)) => {
                    pings_handled += 1;
                    if pings_handled > 5 {
                        let _ = event_tx.send(SioUiEvent::Error("Too many EIO pings during connect".into()));
                        return false;
                    }
                    let pong = format!("3{}", probe);
                    if write.send(Message::Text(pong.into())).await.is_err() {
                        let _ = event_tx.send(SioUiEvent::Error("Write error during connect".into()));
                        return false;
                    }
                    // Re-send SIO CONNECT in case the server expects it after the ping
                    let connect_pkt = format!("4{}", encode_sio_connect(namespace));
                    let _ = write.send(Message::Text(connect_pkt.into())).await;
                }
                _ => {
                    let _ = event_tx.send(SioUiEvent::Error("Unexpected EIO packet during connect".into()));
                    return false;
                }
            },
            _ => {
                let _ = event_tx.send(SioUiEvent::Error("SIO CONNECT ack timeout".into()));
                return false;
            }
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_buffer_evicts_oldest() {
        let mut rb = SioRingBuffer::new(3);
        for i in 0u8..5 {
            rb.push(SioEvent {
                direction: SioDirection::Received,
                namespace: "/".into(),
                event_name: "test".into(),
                payload: i.to_string(),
                ack_id: None,
                is_ack: false,
                timestamp: chrono::Local::now(),
            });
        }
        assert_eq!(rb.len(), 3);
        let payloads: Vec<&str> = rb.iter().map(|e| e.payload.as_str()).collect();
        assert_eq!(payloads, ["2", "3", "4"]);
    }

    #[test]
    fn build_ws_url_http() {
        assert_eq!(
            build_ws_url("http://localhost:3000"),
            "ws://localhost:3000/socket.io/?EIO=4&transport=websocket"
        );
    }

    #[test]
    fn build_ws_url_https() {
        assert_eq!(
            build_ws_url("https://example.com"),
            "wss://example.com/socket.io/?EIO=4&transport=websocket"
        );
    }

    #[test]
    fn build_ws_url_trailing_slash() {
        assert_eq!(
            build_ws_url("http://localhost:3000/"),
            "ws://localhost:3000/socket.io/?EIO=4&transport=websocket"
        );
    }

    #[test]
    fn encode_event_default_ns_no_ack() {
        assert_eq!(
            encode_sio_event("/", "chat", r#"{"msg":"hi"}"#, None),
            r#"2["chat",{"msg":"hi"}]"#
        );
    }

    #[test]
    fn encode_event_custom_ns_with_ack() {
        assert_eq!(
            encode_sio_event("/admin", "kick", r#""user1""#, Some(5)),
            r#"2/admin,5["kick","user1"]"#
        );
    }

    #[test]
    fn parse_event_array_with_payload() {
        let (name, payload) = parse_event_array(r#"["chat",{"msg":"hi"}]"#).unwrap();
        assert_eq!(name, "chat");
        assert_eq!(payload, r#"{"msg":"hi"}"#);
    }

    #[test]
    fn parse_event_array_no_payload() {
        let (name, payload) = parse_event_array(r#"["ping"]"#).unwrap();
        assert_eq!(name, "ping");
        assert_eq!(payload, "null");
    }

    #[test]
    fn parse_sio_header_default_ns_event() {
        let (ptype, ns, ack_id, data) = parse_sio_header(r#"2["chat","hi"]"#).unwrap();
        assert_eq!(ptype, 2);
        assert_eq!(ns, "/");
        assert_eq!(ack_id, None);
        assert_eq!(data, r#"["chat","hi"]"#);
    }

    #[test]
    fn parse_sio_header_custom_ns_with_ack() {
        let (ptype, ns, ack_id, data) = parse_sio_header(r#"2/admin,5["kick","user1"]"#).unwrap();
        assert_eq!(ptype, 2);
        assert_eq!(ns, "/admin");
        assert_eq!(ack_id, Some(5));
        assert_eq!(data, r#"["kick","user1"]"#);
    }
}
