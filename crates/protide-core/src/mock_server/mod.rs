//! Mock HTTP server for API testing

mod routes;
mod server;

pub use routes::{HttpMethod, MockResponse, MockRoute};

use std::net::SocketAddr;
use std::sync::{Arc, Mutex, RwLock};
use tokio::sync::oneshot;

#[derive(Debug)]
pub struct MockServer {
    routes: Arc<RwLock<Vec<MockRoute>>>,
    shutdown_tx: Option<oneshot::Sender<()>>,
    addr: Option<SocketAddr>,
    port: u16,
    record_mode: Arc<std::sync::atomic::AtomicBool>,
    record_target: Arc<RwLock<Option<String>>>,
    recorded_routes: Arc<Mutex<Vec<MockRoute>>>,
}

impl MockServer {
    pub fn new(port: u16) -> Self {
        Self {
            routes: Arc::new(RwLock::new(Vec::new())),
            shutdown_tx: None,
            addr: None,
            port,
            record_mode: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            record_target: Arc::new(RwLock::new(None)),
            recorded_routes: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn is_running(&self) -> bool {
        self.shutdown_tx.is_some()
    }

    pub fn addr(&self) -> Option<SocketAddr> {
        self.addr
    }

    pub fn base_url(&self) -> Option<String> {
        self.addr.map(|addr| format!("http://{}", addr))
    }

    pub fn add_route(&mut self, route: MockRoute) {
        if let Ok(mut routes) = self.routes.write() {
            routes.push(route);
        }
    }

    pub fn remove_route(&mut self, index: usize) {
        if let Ok(mut routes) = self.routes.write()
            && index < routes.len()
        {
            routes.remove(index);
        }
    }

    pub fn routes(&self) -> Vec<MockRoute> {
        self.routes.read().map(|r| r.clone()).unwrap_or_default()
    }

    pub fn update_route(&mut self, index: usize, route: MockRoute) {
        if let Ok(mut routes) = self.routes.write()
            && index < routes.len()
        {
            routes[index] = route;
        }
    }

    pub fn clear_routes(&mut self) {
        if let Ok(mut routes) = self.routes.write() {
            routes.clear();
        }
    }

    pub fn set_record_mode(&mut self, enabled: bool, target: Option<String>) {
        self.record_mode
            .store(enabled, std::sync::atomic::Ordering::Relaxed);
        if let Ok(mut t) = self.record_target.write() {
            *t = target;
        }
    }

    pub fn is_recording(&self) -> bool {
        self.record_mode.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn record_target(&self) -> Option<String> {
        self.record_target.read().ok().and_then(|t| t.clone())
    }

    pub fn drain_recorded(&mut self) -> Vec<MockRoute> {
        self.recorded_routes
            .lock()
            .map(|mut r| std::mem::take(&mut *r))
            .unwrap_or_default()
    }

    pub fn start(&mut self) -> Result<SocketAddr, String> {
        if self.is_running() {
            return Err("Server already running".to_string());
        }

        let routes = self.routes.clone();
        let record_mode = self.record_mode.clone();
        let record_target = self.record_target.clone(); // Arc clone - handler sees live updates
        let recorded_routes = self.recorded_routes.clone();
        let port = self.port;
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let (addr_tx, addr_rx) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(e) => {
                    let _ = addr_tx.send(Err(format!("Failed to create tokio runtime: {}", e)));
                    return;
                }
            };

            rt.block_on(async move {
                let app =
                    server::create_router(routes, record_mode, record_target, recorded_routes);
                let addr = SocketAddr::from(([127, 0, 0, 1], port));

                let listener = match tokio::net::TcpListener::bind(addr).await {
                    Ok(l) => l,
                    Err(e) => {
                        let _ = addr_tx.send(Err(e.to_string()));
                        return;
                    }
                };

                let actual_addr = match listener.local_addr() {
                    Ok(addr) => addr,
                    Err(e) => {
                        let _ = addr_tx.send(Err(format!("Failed to get local address: {}", e)));
                        return;
                    }
                };
                let _ = addr_tx.send(Ok(actual_addr));

                axum::serve(listener, app)
                    .with_graceful_shutdown(async {
                        let _ = shutdown_rx.await;
                    })
                    .await
                    .ok();
            });
        });

        match addr_rx.recv_timeout(std::time::Duration::from_secs(5)) {
            Ok(Ok(addr)) => {
                self.shutdown_tx = Some(shutdown_tx);
                self.addr = Some(addr);
                Ok(addr)
            }
            Ok(Err(e)) => Err(e),
            Err(_) => Err("Server startup timeout".to_string()),
        }
    }

    pub fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        self.addr = None;
    }
}

impl Default for MockServer {
    fn default() -> Self {
        Self::new(8080)
    }
}

impl Drop for MockServer {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every test request is capped so a hung proxy/route fails the test
    /// rather than parking the suite on an unbounded socket read.
    fn test_client() -> reqwest::blocking::Client {
        reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .connect_timeout(std::time::Duration::from_secs(5))
            .build()
            .expect("client should build")
    }

    #[test]
    fn test_mock_server_creation() {
        let server = MockServer::new(9999);
        assert!(!server.is_running());
        assert_eq!(server.routes().len(), 0);
    }

    #[test]
    fn test_add_route() {
        let mut server = MockServer::new(9999);
        server.add_route(MockRoute::new(
            HttpMethod::Get,
            "/test",
            MockResponse::ok("Hello"),
        ));
        assert_eq!(server.routes().len(), 1);
    }

    #[test]
    fn test_server_start_stop() {
        let mut server = MockServer::new(0);
        server.add_route(MockRoute::new(
            HttpMethod::Get,
            "/health",
            MockResponse::ok(r#"{"status":"ok"}"#).with_header("Content-Type", "application/json"),
        ));

        let result = server.start();
        assert!(result.is_ok());
        assert!(server.is_running());

        let addr = result.unwrap();
        assert!(addr.port() > 0);

        server.stop();
        assert!(!server.is_running());
    }

    /// Regression test for a fixed bug in `server.rs`'s proxy URL construction:
    /// the forwarded URL used to be built as `format!("{target}{path}{query}")`
    /// where `query` comes from `Uri::query()` (which does NOT include a leading
    /// `?`). That meant a request like `GET /echo?foo=bar` was forwarded upstream
    /// as `GET /echofoo=bar` instead of `GET /echo?foo=bar` - the query string got
    /// spliced directly onto the path, corrupting the target route entirely (not
    /// just losing the query params). `server.rs` now re-attaches the `?`
    /// separator via `build_proxy_url`, so the upstream mock server (which only
    /// defines route `GET /echo`) correctly matches and returns 200.
    #[test]
    fn test_proxy_query_string_is_forwarded_correctly() {
        let mut target = MockServer::new(0);
        target.add_route(MockRoute::new(
            HttpMethod::Get,
            "/echo",
            MockResponse::ok("OK-ECHO"),
        ));
        let target_addr = target.start().expect("target server should start");
        let target_url = format!("http://{}", target_addr);

        let mut proxy = MockServer::new(0);
        proxy.add_route(MockRoute::proxy(HttpMethod::Get, "/echo", &target_url));
        let proxy_addr = proxy.start().expect("proxy server should start");

        let resp = test_client()
            .get(format!("http://{}/echo?foo=bar", proxy_addr))
            .send()
            .expect("request to proxy should succeed at the transport level");

        // Correct behavior forwards to target as `/echo?foo=bar` and gets 200.
        assert_eq!(
            resp.status(),
            200,
            "query string should be forwarded correctly, not merged into the path"
        );
        let body = resp.text().unwrap_or_default();
        assert_eq!(body, "OK-ECHO");

        proxy.stop();
        target.stop();
    }

    /// Regression test for the proxy hop-count guard in `server.rs`: two real
    /// mock server instances are configured to proxy `/loop` at each other,
    /// forming a genuine cycle. Without a hop limit this would recurse forever
    /// (unbounded nested outbound connections). The guard rejects the request
    /// with 508 Loop Detected once `MAX_PROXY_HOPS` (5) is exceeded, so this
    /// test is bounded by the fix itself and cannot hang or spawn unbounded
    /// connections.
    #[test]
    fn test_proxy_hop_limit_prevents_infinite_loop() {
        let mut server_a = MockServer::new(0);
        let mut server_b = MockServer::new(0);

        // Placeholder targets - the real target URL needs the other server's
        // port, which is only known after both servers have started.
        server_a.add_route(MockRoute::proxy(
            HttpMethod::Get,
            "/loop",
            "http://placeholder",
        ));
        server_b.add_route(MockRoute::proxy(
            HttpMethod::Get,
            "/loop",
            "http://placeholder",
        ));

        let addr_a = server_a.start().expect("server A should start");
        let addr_b = server_b.start().expect("server B should start");

        server_a.update_route(
            0,
            MockRoute::proxy(HttpMethod::Get, "/loop", format!("http://{}", addr_b)),
        );
        server_b.update_route(
            0,
            MockRoute::proxy(HttpMethod::Get, "/loop", format!("http://{}", addr_a)),
        );

        let resp = test_client()
            .get(format!("http://{}/loop", addr_a))
            .send()
            .expect("request should complete promptly instead of hanging, thanks to the hop-count guard");

        assert_eq!(
            resp.status().as_u16(),
            508,
            "hop-count guard should reject the request once the proxy loop exceeds the hop limit"
        );

        server_a.stop();
        server_b.stop();
    }
}
