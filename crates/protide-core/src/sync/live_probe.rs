use std::net::{SocketAddr, UdpSocket};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

use super::types::{LiveActivity, NodeId};

const DEFAULT_PORT: u16 = 42069;
const BROADCAST_ADDR: &str = "255.255.255.255";
const MAGIC_PREAMBLE: &[u8] = b"PROTIDE_LIVE";
/// Read buffer size of the listener thread. Datagrams larger than this are
/// truncated by `recv_from` and therefore fail to decode.
const MAX_PACKET: usize = 2048;

/// Frame an activity as a wire packet: `MAGIC_PREAMBLE || JSON`.
fn encode_packet(activity: &LiveActivity) -> Result<Vec<u8>, String> {
    let payload =
        serde_json::to_vec(activity).map_err(|e| format!("Failed to serialize activity: {}", e))?;
    let mut packet = MAGIC_PREAMBLE.to_vec();
    packet.extend_from_slice(&payload);
    Ok(packet)
}

/// Parse a datagram back into a `LiveActivity`, rejecting anything that is not
/// a well-formed Protide live packet. Never panics on hostile input.
fn decode_packet(datagram: &[u8]) -> Option<LiveActivity> {
    let payload = datagram.strip_prefix(MAGIC_PREAMBLE)?;
    serde_json::from_slice(payload).ok()
}

/// UDP broadcast-based live activity sharing for local network collaboration.
///
/// Zero-configuration - peers on the same subnet automatically discover
/// each other's live request activity. No servers, no accounts.
pub struct LiveProbe {
    socket: UdpSocket,
    node_id: NodeId,
    node_name: String,
    running: Arc<AtomicBool>,
    activity_rx: Receiver<(SocketAddr, LiveActivity)>,
    _reader: Option<thread::JoinHandle<()>>,
}

impl LiveProbe {
    /// Start a live probe on the specified port (or default 42069).
    pub fn start(node_id: NodeId, node_name: String, port: Option<u16>) -> Result<Self, String> {
        let port = port.unwrap_or(DEFAULT_PORT);
        let bind_addr: SocketAddr = format!("0.0.0.0:{}", port)
            .parse()
            .map_err(|e| format!("Invalid bind address: {}", e))?;

        let socket =
            UdpSocket::bind(bind_addr).map_err(|e| format!("Failed to bind UDP socket: {}", e))?;

        socket
            .set_broadcast(true)
            .map_err(|e| format!("Failed to set broadcast: {}", e))?;

        socket
            .set_read_timeout(Some(Duration::from_secs(1)))
            .map_err(|e| format!("Failed to set read timeout: {}", e))?;

        let (activity_tx, activity_rx) = mpsc::channel::<(SocketAddr, LiveActivity)>();
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();

        let reader_socket = socket
            .try_clone()
            .map_err(|e| format!("Failed to clone socket: {}", e))?;

        // Spawn a reader thread that listens for broadcasts
        let _reader = thread::Builder::new()
            .name("protide-live-probe".into())
            .spawn(move || {
                let mut buf = [0u8; MAX_PACKET];
                while running_clone.load(Ordering::Relaxed) {
                    match reader_socket.recv_from(&mut buf) {
                        Ok((len, src)) => {
                            if let Some(activity) = decode_packet(&buf[..len]) {
                                let _ = activity_tx.send((src, activity));
                            }
                        }
                        Err(_) => continue,
                    }
                }
            })
            .map_err(|e| format!("Failed to spawn reader thread: {}", e))?;

        Ok(Self {
            socket,
            node_id,
            node_name,
            running,
            activity_rx,
            _reader: Some(_reader),
        })
    }

    /// Broadcast a live activity to all peers on the local subnet.
    pub fn broadcast(
        &self,
        request_name: &str,
        status: u16,
        time_ms: u64,
        method: &str,
        url: &str,
    ) -> Result<(), String> {
        let activity = LiveActivity {
            node_id: self.node_id.0.clone(),
            node_name: self.node_name.clone(),
            request_name: request_name.to_string(),
            status,
            time_ms,
            method: method.to_string(),
            url: url.to_string(),
        };

        let packet = encode_packet(&activity)?;

        let local_addr = self
            .socket
            .local_addr()
            .unwrap_or(([0, 0, 0, 0], DEFAULT_PORT).into());
        let broadcast_addr: SocketAddr = format!("{}:{}", BROADCAST_ADDR, local_addr.port())
            .parse()
            .map_err(|_| "Invalid broadcast address".to_string())?;

        self.socket
            .send_to(&packet, broadcast_addr)
            .map_err(|e| format!("Failed to send broadcast: {}", e))?;

        Ok(())
    }

    /// Drain received activities (non-blocking)
    pub fn drain_activities(&self) -> Vec<(SocketAddr, LiveActivity)> {
        let mut activities = Vec::new();
        while let Ok(activity) = self.activity_rx.try_recv() {
            activities.push(activity);
        }
        activities
    }
}

impl Drop for LiveProbe {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    fn sample(url: &str) -> LiveActivity {
        LiveActivity {
            node_id: "node-1".into(),
            node_name: "alice".into(),
            request_name: "Get users".into(),
            status: 200,
            time_ms: 42,
            method: "GET".into(),
            url: url.into(),
        }
    }

    fn assert_same(a: &LiveActivity, b: &LiveActivity) {
        assert_eq!(a.node_id, b.node_id);
        assert_eq!(a.node_name, b.node_name);
        assert_eq!(a.request_name, b.request_name);
        assert_eq!(a.status, b.status);
        assert_eq!(a.time_ms, b.time_ms);
        assert_eq!(a.method, b.method);
        assert_eq!(a.url, b.url);
    }

    #[test]
    fn test_packet_roundtrip() {
        let activity = sample("https://api.example.com/users?q=1");
        let packet = encode_packet(&activity).unwrap();
        assert!(packet.starts_with(MAGIC_PREAMBLE));
        assert_same(&decode_packet(&packet).unwrap(), &activity);
    }

    /// Non-ASCII names/URLs must survive the JSON framing intact.
    #[test]
    fn test_packet_roundtrip_unicode() {
        let mut activity = sample("https://例え.jp/パス");
        activity.node_name = "アリス 🎈".into();
        let packet = encode_packet(&activity).unwrap();
        assert_same(&decode_packet(&packet).unwrap(), &activity);
    }

    /// Anything that is not our protocol must be ignored, never decoded and
    /// never panic - this is unauthenticated data from any host on the subnet.
    #[test]
    fn test_decode_rejects_foreign_and_malformed_datagrams() {
        let cases: Vec<Vec<u8>> = vec![
            Vec::new(),                                     // empty datagram
            b"PROTIDE_LIV".to_vec(),                        // truncated preamble
            b"protide_live{}".to_vec(),                     // wrong case
            MAGIC_PREAMBLE.to_vec(),                        // preamble, no payload
            [MAGIC_PREAMBLE, b"not json"].concat(),         // preamble + garbage
            [MAGIC_PREAMBLE, b"{\"node_id\":1}"].concat(),  // wrong field type
            [MAGIC_PREAMBLE, b"{}"].concat(),               // missing every field
            [MAGIC_PREAMBLE, &[0xff, 0xfe, 0xfd]].concat(), // invalid utf-8
            [b"XX".as_slice(), MAGIC_PREAMBLE].concat(),    // preamble not at offset 0
        ];
        for case in cases {
            assert!(
                decode_packet(&case).is_none(),
                "unexpectedly decoded {:?}",
                case
            );
        }
    }

    /// A valid packet chopped at any length must fail cleanly rather than
    /// yielding a half-populated activity or panicking.
    #[test]
    fn test_decode_truncated_valid_packet_never_panics() {
        let packet = encode_packet(&sample("https://api.example.com")).unwrap();
        for len in 0..packet.len() {
            assert!(
                decode_packet(&packet[..len]).is_none(),
                "truncation to {} bytes decoded",
                len
            );
        }
        assert!(decode_packet(&packet).is_some());
    }

    /// Trailing bytes after the JSON payload are rejected, so a packet cannot
    /// be padded with attacker-chosen data and still parse.
    #[test]
    fn test_decode_rejects_trailing_garbage() {
        let mut packet = encode_packet(&sample("https://api.example.com")).unwrap();
        packet.extend_from_slice(b"trailing");
        assert!(decode_packet(&packet).is_none());
    }

    /// An activity whose JSON exceeds the listener's read buffer is silently
    /// dropped by the receiver (recv_from truncates); it must not decode into
    /// a partial activity.
    #[test]
    fn test_oversized_packet_is_dropped_not_corrupted() {
        let packet = encode_packet(&sample(&"x".repeat(4096))).unwrap();
        assert!(packet.len() > MAX_PACKET);
        assert!(decode_packet(&packet[..MAX_PACKET]).is_none());
    }

    /// End-to-end over a loopback socket pair: the exact bytes `broadcast`
    /// puts on the wire are what the listener thread's decode path accepts,
    /// using the same buffer size. Loopback only - no broadcast, no fixed port.
    #[test]
    fn test_packet_survives_loopback_udp() {
        let listener = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        listener
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let sender = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();

        let activity = sample("https://api.example.com/loopback");
        let packet = encode_packet(&activity).unwrap();
        sender
            .send_to(&packet, listener.local_addr().unwrap())
            .unwrap();

        let mut buf = [0u8; MAX_PACKET];
        let (len, src) = listener.recv_from(&mut buf).unwrap();
        assert_eq!(src.ip(), Ipv4Addr::LOCALHOST);
        assert_same(&decode_packet(&buf[..len]).unwrap(), &activity);
    }
}
