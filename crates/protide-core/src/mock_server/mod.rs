//! Mock HTTP server for API testing

mod routes;
mod server;

pub use routes::{MockRoute, MockResponse, HttpMethod};

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
            && index < routes.len() {
                routes.remove(index);
            }
    }

    pub fn routes(&self) -> Vec<MockRoute> {
        self.routes.read().map(|r| r.clone()).unwrap_or_default()
    }

    pub fn update_route(&mut self, index: usize, route: MockRoute) {
        if let Ok(mut routes) = self.routes.write()
            && index < routes.len() {
                routes[index] = route;
            }
    }

    pub fn clear_routes(&mut self) {
        if let Ok(mut routes) = self.routes.write() {
            routes.clear();
        }
    }

    pub fn set_record_mode(&mut self, enabled: bool, target: Option<String>) {
        self.record_mode.store(enabled, std::sync::atomic::Ordering::Relaxed);
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
        self.recorded_routes.lock().map(|mut r| std::mem::take(&mut *r)).unwrap_or_default()
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
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create tokio runtime");

            rt.block_on(async move {
                let app = server::create_router(routes, record_mode, record_target, recorded_routes);
                let addr = SocketAddr::from(([127, 0, 0, 1], port));

                let listener = match tokio::net::TcpListener::bind(addr).await {
                    Ok(l) => l,
                    Err(e) => {
                        let _ = addr_tx.send(Err(e.to_string()));
                        return;
                    }
                };

                let actual_addr = listener.local_addr().unwrap();
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
}
