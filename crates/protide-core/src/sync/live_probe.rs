use std::net::{SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use super::types::{LiveActivity, NodeId};

const DEFAULT_PORT: u16 = 42069;
const BROADCAST_ADDR: &str = "255.255.255.255";
const MAGIC_PREAMBLE: &[u8] = b"PROTIDE_LIVE";

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
    pub fn start(
        node_id: NodeId,
        node_name: String,
        port: Option<u16>,
    ) -> Result<Self, String> {
        let port = port.unwrap_or(DEFAULT_PORT);
        let bind_addr: SocketAddr = format!("0.0.0.0:{}", port)
            .parse()
            .map_err(|e| format!("Invalid bind address: {}", e))?;

        let socket = UdpSocket::bind(bind_addr)
            .map_err(|e| format!("Failed to bind UDP socket: {}", e))?;

        socket.set_broadcast(true)
            .map_err(|e| format!("Failed to set broadcast: {}", e))?;

        socket.set_read_timeout(Some(Duration::from_secs(1)))
            .map_err(|e| format!("Failed to set read timeout: {}", e))?;

        let (activity_tx, activity_rx) = mpsc::channel::<(SocketAddr, LiveActivity)>();
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();

        let reader_socket = socket.try_clone()
            .map_err(|e| format!("Failed to clone socket: {}", e))?;

        // Spawn a reader thread that listens for broadcasts
        let _reader = thread::Builder::new()
            .name("protide-live-probe".into())
            .spawn(move || {
                let mut buf = [0u8; 2048];
                while running_clone.load(Ordering::Relaxed) {
                    match reader_socket.recv_from(&mut buf) {
                        Ok((len, src)) => {
                            let msg = &buf[..len];
                            if msg.starts_with(MAGIC_PREAMBLE) {
                                let payload = &msg[MAGIC_PREAMBLE.len()..];
                                if let Ok(activity) = serde_json::from_slice::<LiveActivity>(payload) {
                                    let _ = activity_tx.send((src, activity));
                                }
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
    pub fn broadcast(&self, request_name: &str, status: u16, time_ms: u64, method: &str, url: &str) -> Result<(), String> {
        let activity = LiveActivity {
            node_id: self.node_id.0.clone(),
            node_name: self.node_name.clone(),
            request_name: request_name.to_string(),
            status,
            time_ms,
            method: method.to_string(),
            url: url.to_string(),
        };

        let payload = serde_json::to_vec(&activity)
            .map_err(|e| format!("Failed to serialize activity: {}", e))?;

        let mut packet = MAGIC_PREAMBLE.to_vec();
        packet.extend_from_slice(&payload);

        let local_addr = self.socket.local_addr().unwrap_or(([0, 0, 0, 0], DEFAULT_PORT).into());
        let broadcast_addr: SocketAddr = format!("{}:{}", BROADCAST_ADDR, local_addr.port())
            .parse()
            .map_err(|_| "Invalid broadcast address".to_string())?;

        self.socket.send_to(&packet, broadcast_addr)
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
