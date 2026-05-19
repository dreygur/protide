use std::sync::{Arc, Mutex, RwLock};
use super::routes::{HttpMethod, MockResponse, MockRoute};

pub(super) struct RouterState {
    pub routes: Arc<RwLock<Vec<MockRoute>>>,
    pub record_mode: Arc<std::sync::atomic::AtomicBool>,
    pub record_target: Arc<RwLock<Option<String>>>,
    pub recorded_routes: Arc<Mutex<Vec<MockRoute>>>,
}

pub(super) fn create_router(
    routes: Arc<RwLock<Vec<MockRoute>>>,
    record_mode: Arc<std::sync::atomic::AtomicBool>,
    record_target: Arc<RwLock<Option<String>>>,
    recorded_routes: Arc<Mutex<Vec<MockRoute>>>,
) -> axum::Router {
    use axum::{
        body::Body,
        extract::State,
        http::{Request, StatusCode},
        response::IntoResponse,
        routing::any,
    };
    use std::sync::atomic::Ordering;

    async fn handler(
        State(state): State<Arc<RouterState>>,
        req: Request<Body>,
    ) -> impl IntoResponse {
        let method = req.method().to_string();
        let path = req.uri().path().to_string();
        let query = req.uri().query().unwrap_or("").to_string();

        let req_headers: Vec<(String, String)> = req
            .headers()
            .iter()
            .filter_map(|(k, v)| {
                let key = k.as_str().to_lowercase();
                if matches!(key.as_str(), "host" | "connection" | "transfer-encoding") {
                    return None;
                }
                Some((k.to_string(), v.to_str().unwrap_or("").to_string()))
            })
            .collect();

        let body_bytes = axum::body::to_bytes(req.into_body(), 16 * 1024 * 1024)
            .await
            .unwrap_or_default();

        let matched = {
            let routes_guard = state.routes.read().unwrap();
            routes_guard.iter().find(|r| r.matches(&method, &path)).cloned()
        };

        let recording = state.record_mode.load(Ordering::Relaxed);
        let live_target = state.record_target.read().ok().and_then(|t| t.clone());
        if recording {
            if let Some(ref target) = live_target {
                let full_url = if query.is_empty() {
                    format!("{}{}", target.trim_end_matches('/'), path)
                } else {
                    format!("{}{}{}", target.trim_end_matches('/'), path, query)
                };
                let resp = proxy_request(&method, &full_url, req_headers, body_bytes.to_vec()).await;
                let status = resp.status().as_u16();
                let body_clone = axum::body::to_bytes(resp.into_body(), 16 * 1024 * 1024)
                    .await
                    .unwrap_or_default();
                let body_str = String::from_utf8_lossy(&body_clone).into_owned();
                let method_enum = match method.to_uppercase().as_str() {
                    "GET" => HttpMethod::Get,
                    "POST" => HttpMethod::Post,
                    "PUT" => HttpMethod::Put,
                    "PATCH" => HttpMethod::Patch,
                    "DELETE" => HttpMethod::Delete,
                    "HEAD" => HttpMethod::Head,
                    "OPTIONS" => HttpMethod::Options,
                    _ => HttpMethod::Any,
                };
                let new_route = MockRoute::new(method_enum, &path, MockResponse::new(status, body_str.clone()));
                if let Ok(mut recorded) = state.recorded_routes.lock() {
                    recorded.push(new_route);
                }
                let mut builder = axum::http::Response::builder().status(status);
                builder = builder.header("Content-Type", "application/json");
                return builder.body(Body::from(body_str)).unwrap_or_else(|_| {
                    axum::http::Response::builder().status(502).body(Body::from("Record proxy error")).unwrap()
                });
            }
        }

        if let Some(route) = matched {
            if let Some(ref target) = route.proxy_target {
                let full_url = if query.is_empty() {
                    format!("{}{}", target.trim_end_matches('/'), path)
                } else {
                    format!("{}{}{}", target.trim_end_matches('/'), path, query)
                };
                return proxy_request(&method, &full_url, req_headers, body_bytes.to_vec())
                    .await
                    .into_response();
            }

            if route.response.delay_ms > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(route.response.delay_ms)).await;
            }

            let response = &route.response;
            let mut builder = axum::http::Response::builder().status(response.status);
            for (key, value) in &response.headers {
                builder = builder.header(key, value);
            }
            return builder.body(Body::from(response.body.clone()))
                .unwrap_or_else(|_| axum::http::Response::builder().status(500).body(Body::from("invalid mock route headers")).unwrap())
                .into_response();
        }

        (StatusCode::NOT_FOUND, "No matching mock route").into_response()
    }

    let state = Arc::new(RouterState { routes, record_mode, record_target, recorded_routes });
    axum::Router::new()
        .route("/{*path}", any(handler))
        .route("/", any(handler))
        .with_state(state)
}

pub(super) async fn proxy_request(
    method: &str,
    url: &str,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
) -> axum::response::Response {
    use axum::{body::Body, http::StatusCode};

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap_or_default();

    let req_method = reqwest::Method::from_bytes(method.as_bytes())
        .unwrap_or(reqwest::Method::GET);

    let mut req = client.request(req_method, url).body(body);
    for (k, v) in &headers {
        req = req.header(k.as_str(), v.as_str());
    }

    match req.send().await {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let mut builder = axum::http::Response::builder().status(status);
            for (k, v) in resp.headers() {
                if let Ok(val) = v.to_str() {
                    builder = builder.header(k.as_str(), val);
                }
            }
            let body_bytes = resp.bytes().await.unwrap_or_default();
            builder
                .body(Body::from(body_bytes))
                .unwrap_or_else(|_| {
                    axum::http::Response::builder()
                        .status(502)
                        .body(Body::from("Proxy response build error"))
                        .unwrap()
                })
        }
        Err(e) => axum::http::Response::builder()
            .status(StatusCode::BAD_GATEWAY)
            .body(Body::from(format!("Proxy error: {}", e)))
            .unwrap(),
    }
}
