//! Test-only loopback HTTP server used by the execution tests.
//!
//! Exercising [`super::http::run_http`] against a real socket is the only way
//! to cover request construction (what actually goes on the wire) end to end.
//! The server always binds port 0 on 127.0.0.1, never talks to the network,
//! and every blocking socket operation is capped by [`SOCKET_TIMEOUT`] so a
//! misbehaving test fails loudly instead of parking the suite forever.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Upper bound on every blocking socket operation performed by the server.
const SOCKET_TIMEOUT: Duration = Duration::from_secs(10);

/// A request as it was actually received on the wire.
#[derive(Debug, Clone, Default)]
pub struct RecordedRequest {
    pub method: String,
    /// Request target, i.e. path + query exactly as sent.
    pub target: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

impl RecordedRequest {
    /// First value for `name` (case-insensitive).
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    /// Every value for `name` (case-insensitive), in wire order.
    pub fn header_values(&self, name: &str) -> Vec<&str> {
        self.headers
            .iter()
            .filter(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
            .collect()
    }
}

/// Build a raw HTTP/1.1 response with `Connection: close` and a correct
/// `Content-Length`, so the client never has to wait for an idle timeout.
pub fn response(status: u16, headers: &[(&str, &str)], body: &str) -> String {
    let mut out = format!(
        "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nConnection: close\r\n",
        status,
        super::http::status_text(status),
        body.len()
    );
    for (k, v) in headers {
        out.push_str(&format!("{}: {}\r\n", k, v));
    }
    out.push_str("\r\n");
    out.push_str(body);
    out
}

/// `200 OK` with a JSON body.
pub fn json_response(body: &str) -> String {
    response(200, &[("Content-Type", "application/json")], body)
}

/// A one-shot loopback HTTP server. Serves one canned response per accepted
/// connection, in order, then stops accepting.
pub struct TestServer {
    base: String,
    recorded: Arc<Mutex<Vec<RecordedRequest>>>,
}

impl TestServer {
    /// Serve `responses[i]` on the i-th connection. An empty string means
    /// "read the request, then close the connection without replying",
    /// which is how the transport-error path is exercised.
    pub fn spawn(responses: Vec<String>) -> Self {
        Self::start(responses, Duration::ZERO)
    }

    /// Server that accepts a connection and then holds it open without ever
    /// replying, so the client's own timeout is what ends the request.
    pub fn stalled() -> Self {
        Self::start(vec![String::new()], Duration::from_secs(3))
    }

    fn start(responses: Vec<String>, hold: Duration) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let base = format!("http://{}", listener.local_addr().expect("addr"));
        // std has no accept timeout, so poll a non-blocking listener against
        // a deadline instead of blocking forever on a client that never comes.
        listener.set_nonblocking(true).expect("set_nonblocking");
        let recorded = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&recorded);

        std::thread::spawn(move || {
            let deadline = Instant::now() + SOCKET_TIMEOUT;
            for reply in responses {
                let mut stream = loop {
                    match listener.accept() {
                        Ok((stream, _)) => break stream,
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            if Instant::now() >= deadline {
                                return;
                            }
                            std::thread::sleep(Duration::from_millis(2));
                        }
                        Err(_) => return,
                    }
                };
                // The accepted socket can inherit the listener's O_NONBLOCK.
                let _ = stream.set_nonblocking(false);
                let _ = stream.set_read_timeout(Some(SOCKET_TIMEOUT));
                let _ = stream.set_write_timeout(Some(SOCKET_TIMEOUT));

                if let Some(req) = read_request(&mut stream) {
                    sink.lock().expect("recorded lock").push(req);
                }
                if reply.is_empty() {
                    std::thread::sleep(hold);
                } else {
                    let _ = stream.write_all(reply.as_bytes());
                    let _ = stream.flush();
                }
            }
        });

        Self { base, recorded }
    }

    /// Server that answers a single request with `200 OK` + JSON.
    pub fn json(body: &str) -> Self {
        Self::spawn(vec![json_response(body)])
    }

    /// Absolute URL for `path` (which must start with `/`).
    pub fn url(&self, path: &str) -> String {
        format!("{}{}", self.base, path)
    }

    /// Requests received so far. Each is recorded before its response is
    /// written, so every request the client has seen answered is present.
    pub fn requests(&self) -> Vec<RecordedRequest> {
        self.recorded.lock().expect("recorded lock").clone()
    }

    /// The single request this server received. Panics unless there is
    /// exactly one, which keeps "the client silently retried" from passing.
    pub fn only_request(&self) -> RecordedRequest {
        let mut reqs = self.requests();
        assert_eq!(reqs.len(), 1, "expected exactly one request");
        reqs.remove(0)
    }
}

/// Read one HTTP request: the head, then a `Content-Length` or chunked body.
fn read_request(stream: &mut std::net::TcpStream) -> Option<RecordedRequest> {
    let mut raw: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 4096];

    let head_end = loop {
        if let Some(pos) = find(&raw, b"\r\n\r\n") {
            break pos + 4;
        }
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => return None,
            Ok(n) => raw.extend_from_slice(&chunk[..n]),
        }
    };

    let head = String::from_utf8_lossy(&raw[..head_end]).to_string();
    let mut lines = head.lines();
    let mut parts = lines.next()?.split_whitespace();
    let method = parts.next()?.to_string();
    let target = parts.next()?.to_string();

    let headers: Vec<(String, String)> = lines
        .filter(|l| !l.is_empty())
        .filter_map(|l| {
            let (k, v) = l.split_once(':')?;
            Some((k.trim().to_string(), v.trim().to_string()))
        })
        .collect();

    let lookup = |name: &str| {
        headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    };
    let content_length: usize = lookup("content-length")
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(0);
    let chunked = lookup("transfer-encoding").is_some_and(|v| v.contains("chunked"));

    // Body: either exactly Content-Length bytes, or chunked up to the
    // terminating zero-length chunk. Both are bounded by the read timeout.
    while (chunked && !raw.ends_with(b"0\r\n\r\n"))
        || (!chunked && raw.len() < head_end + content_length)
    {
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(n) => raw.extend_from_slice(&chunk[..n]),
        }
    }

    Some(RecordedRequest {
        method,
        target,
        headers,
        body: String::from_utf8_lossy(&raw[head_end..]).to_string(),
    })
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}
