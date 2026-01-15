//! Mock HTTP server for API testing
//!
//! Provides a local HTTP server that can be configured with mock responses.

mod routes;

pub use routes::{MockRoute, MockResponse, HttpMethod};

use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use tokio::sync::oneshot;

/// Mock server state
#[derive(Debug)]
pub struct MockServer {
    /// Configured routes
    routes: Arc<RwLock<Vec<MockRoute>>>,
    /// Server shutdown signal
    shutdown_tx: Option<oneshot::Sender<()>>,
    /// Server address when running
    addr: Option<SocketAddr>,
    /// Port to run on
    port: u16,
}

impl MockServer {
    /// Create a new mock server
    pub fn new(port: u16) -> Self {
        Self {
            routes: Arc::new(RwLock::new(Vec::new())),
            shutdown_tx: None,
            addr: None,
            port,
        }
    }

    /// Check if server is running
    pub fn is_running(&self) -> bool {
        self.shutdown_tx.is_some()
    }

    /// Get the server address
    pub fn addr(&self) -> Option<SocketAddr> {
        self.addr
    }

    /// Get the base URL
    pub fn base_url(&self) -> Option<String> {
        self.addr.map(|addr| format!("http://{}", addr))
    }

    /// Add a route
    pub fn add_route(&mut self, route: MockRoute) {
        if let Ok(mut routes) = self.routes.write() {
            routes.push(route);
        }
    }

    /// Remove a route by index
    pub fn remove_route(&mut self, index: usize) {
        if let Ok(mut routes) = self.routes.write() {
            if index < routes.len() {
                routes.remove(index);
            }
        }
    }

    /// Get all routes
    pub fn routes(&self) -> Vec<MockRoute> {
        self.routes.read().map(|r| r.clone()).unwrap_or_default()
    }

    /// Update a route
    pub fn update_route(&mut self, index: usize, route: MockRoute) {
        if let Ok(mut routes) = self.routes.write() {
            if index < routes.len() {
                routes[index] = route;
            }
        }
    }

    /// Clear all routes
    pub fn clear_routes(&mut self) {
        if let Ok(mut routes) = self.routes.write() {
            routes.clear();
        }
    }

    /// Start the server
    pub fn start(&mut self) -> Result<SocketAddr, String> {
        if self.is_running() {
            return Err("Server already running".to_string());
        }

        let routes = self.routes.clone();
        let port = self.port;
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let (addr_tx, addr_rx) = std::sync::mpsc::channel();

        // Spawn server in background thread with its own tokio runtime
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create tokio runtime");

            rt.block_on(async move {
                let app = create_router(routes);
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

        // Wait for server to start
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

    /// Stop the server
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

/// Create the axum router
fn create_router(routes: Arc<RwLock<Vec<MockRoute>>>) -> axum::Router {
    use axum::{
        body::Body,
        extract::State,
        http::{Request, StatusCode},
        response::IntoResponse,
        routing::any,
    };

    async fn handler(
        State(routes): State<Arc<RwLock<Vec<MockRoute>>>>,
        req: Request<Body>,
    ) -> impl IntoResponse {
        let method = req.method().to_string();
        let path = req.uri().path().to_string();

        // Find matching route
        let routes_guard = routes.read().unwrap();
        for route in routes_guard.iter() {
            if route.matches(&method, &path) {
                let response = &route.response;
                let mut builder = axum::http::Response::builder()
                    .status(response.status);

                for (key, value) in &response.headers {
                    builder = builder.header(key, value);
                }

                return builder
                    .body(Body::from(response.body.clone()))
                    .unwrap()
                    .into_response();
            }
        }

        // No match found
        (StatusCode::NOT_FOUND, "No matching mock route").into_response()
    }

    axum::Router::new()
        .route("/{*path}", any(handler))
        .route("/", any(handler))
        .with_state(routes)
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
        let mut server = MockServer::new(0); // Use port 0 for random available port
        server.add_route(MockRoute::new(
            HttpMethod::Get,
            "/health",
            MockResponse::ok(r#"{"status":"ok"}"#).with_header("Content-Type", "application/json"),
        ));

        // Start server
        let result = server.start();
        assert!(result.is_ok());
        assert!(server.is_running());

        let addr = result.unwrap();
        assert!(addr.port() > 0);

        // Stop server
        server.stop();
        assert!(!server.is_running());
    }
}
