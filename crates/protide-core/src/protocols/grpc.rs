//! gRPC protocol support using prost-reflect for dynamic proto handling

use prost::Message;
use prost_reflect::{DescriptorPool, DynamicMessage};
use std::path::Path;
use std::time::Duration;

/// gRPC method information
#[derive(Debug, Clone)]
pub struct GrpcMethod {
    pub name: String,
    pub full_name: String,
    pub input_type: String,
    pub output_type: String,
    pub is_client_streaming: bool,
    pub is_server_streaming: bool,
}

/// gRPC service information
#[derive(Debug, Clone)]
pub struct GrpcService {
    pub name: String,
    pub full_name: String,
    pub methods: Vec<GrpcMethod>,
}

/// Parse a .proto file and return a DescriptorPool.
/// Uses protox to compile the file without requiring system protoc.
pub fn parse_proto_file(path: &Path) -> Result<DescriptorPool, String> {
    let dir = path.parent().unwrap_or(Path::new("."));
    let fds = protox::compile([path], [dir])
        .map_err(|e| format!("Proto compile error: {}", e))?;
    DescriptorPool::from_file_descriptor_set(fds)
        .map_err(|e| format!("Descriptor pool error: {}", e))
}

/// Extract services and methods from a descriptor pool.
pub fn extract_services(pool: &DescriptorPool) -> Vec<GrpcService> {
    pool.services()
        .map(|svc| {
            let methods = svc
                .methods()
                .map(|m| GrpcMethod {
                    name: m.name().to_string(),
                    full_name: m.full_name().to_string(),
                    input_type: m.input().full_name().to_string(),
                    output_type: m.output().full_name().to_string(),
                    is_client_streaming: m.is_client_streaming(),
                    is_server_streaming: m.is_server_streaming(),
                })
                .collect();
            GrpcService {
                name: svc.name().to_string(),
                full_name: svc.full_name().to_string(),
                methods,
            }
        })
        .collect()
}

/// Execute a unary gRPC call (blocking).
///
/// `method_full_name` format: `ServiceName/MethodName` or `package.ServiceName/MethodName`
/// (leading slash is optional).
///
/// Returns `(response_json, elapsed)` on success.
pub fn execute_unary_blocking(
    url: &str,
    method_full_name: &str,
    message_json: &str,
    metadata: Vec<(String, String)>,
    proto_path: &Path,
) -> Result<(String, Duration), String> {
    let start = std::time::Instant::now();

    let pool = parse_proto_file(proto_path)?;

    // Parse "Service/Method" or "pkg.Service/Method"
    let method_path = method_full_name.trim_start_matches('/');
    let slash_pos = method_path
        .rfind('/')
        .ok_or_else(|| format!("Invalid method name '{}': missing '/'", method_full_name))?;
    let service_name = &method_path[..slash_pos];
    let method_name = &method_path[slash_pos + 1..];

    let service_desc = pool
        .get_service_by_name(service_name)
        .ok_or_else(|| format!("Service not found: '{}'", service_name))?;

    let method_desc = service_desc
        .methods()
        .find(|m| m.name() == method_name)
        .ok_or_else(|| format!("Method not found: '{}'", method_name))?;

    let input_desc = method_desc.input();
    let output_desc = method_desc.output();

    // JSON → DynamicMessage → protobuf bytes
    let request_msg = DynamicMessage::deserialize(
        input_desc,
        &mut serde_json::Deserializer::from_str(message_json),
    )
    .map_err(|e| format!("JSON parse error: {}", e))?;
    let request_bytes = request_msg.encode_to_vec();
    let grpc_body = grpc_encode_message(&request_bytes);

    // Build HTTP URL: grpc:// → http://, grpcs:// → https://
    let http_url = url
        .trim_end_matches('/')
        .replace("grpc://", "http://")
        .replace("grpcs://", "https://");
    let full_url = format!("{}/{}", http_url, method_path);
    let use_h2c = http_url.starts_with("http://");

    // Build reqwest blocking client with HTTP/2
    let mut builder = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30));
    if use_h2c {
        builder = builder.http2_prior_knowledge();
    }
    let client = builder
        .build()
        .map_err(|e| format!("Client build error: {}", e))?;

    let mut req = client
        .post(&full_url)
        .header("content-type", "application/grpc+proto")
        .header("te", "trailers")
        .body(grpc_body);

    for (key, value) in &metadata {
        req = req.header(key.as_str(), value.as_str());
    }

    let response = req
        .send()
        .map_err(|e| format!("gRPC request failed: {}", e))?;

    let elapsed = start.elapsed();

    // Check grpc-status (status 0 = OK)
    let grpc_status = response
        .headers()
        .get("grpc-status")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(0);

    if grpc_status != 0 {
        let grpc_message = response
            .headers()
            .get("grpc-message")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("Unknown gRPC error")
            .to_string();
        return Err(format!(
            "gRPC error status {}: {}",
            grpc_status, grpc_message
        ));
    }

    let body_bytes = response
        .bytes()
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    let msg_bytes = grpc_decode_message(&body_bytes)?;

    let response_msg = DynamicMessage::decode(output_desc, msg_bytes.as_ref())
        .map_err(|e| format!("Protobuf decode error: {}", e))?;

    let response_json = serde_json::to_string_pretty(&response_msg)
        .map_err(|e| format!("JSON serialize error: {}", e))?;

    Ok((response_json, elapsed))
}

fn grpc_encode_message(msg_bytes: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(5 + msg_bytes.len());
    buf.push(0u8); // compression flag: not compressed
    buf.extend_from_slice(&(msg_bytes.len() as u32).to_be_bytes());
    buf.extend_from_slice(msg_bytes);
    buf
}

fn grpc_decode_message(data: &[u8]) -> Result<Vec<u8>, String> {
    if data.len() < 5 {
        return Err(format!(
            "gRPC response too short ({} bytes, need at least 5)",
            data.len()
        ));
    }
    let _compressed = data[0];
    let msg_len = u32::from_be_bytes([data[1], data[2], data[3], data[4]]) as usize;
    if data.len() < 5 + msg_len {
        return Err(format!(
            "Incomplete gRPC response (got {} bytes, expected {})",
            data.len(),
            5 + msg_len
        ));
    }
    Ok(data[5..5 + msg_len].to_vec())
}

/// Streaming response chunk
#[derive(Debug, Clone)]
pub struct StreamingChunk {
    pub data: String,
    pub is_final: bool,
    pub is_error: bool,
}

impl StreamingChunk {
    fn error(msg: String) -> Self {
        Self {
            data: msg,
            is_final: true,
            is_error: true,
        }
    }

    fn data(data: String, is_final: bool) -> Self {
        Self {
            data,
            is_final,
            is_error: false,
        }
    }
}

/// Execute server streaming gRPC using async HTTP/2.
/// Returns a vector of response chunks (JSON strings).
pub async fn execute_server_streaming(
    url: &str,
    method_full_name: &str,
    message_json: &str,
    metadata: Vec<(String, String)>,
    proto_path: &Path,
) -> Result<Vec<String>, String> {
    let pool = parse_proto_file(proto_path)?;

    let method_path = method_full_name.trim_start_matches('/');
    let slash_pos = method_path
        .rfind('/')
        .ok_or_else(|| format!("Invalid method name '{}': missing '/'", method_full_name))?;
    let service_name = &method_path[..slash_pos];
    let method_name = &method_path[slash_pos + 1..];

    let service_desc = pool
        .get_service_by_name(service_name)
        .ok_or_else(|| format!("Service not found: '{}'", service_name))?;

    let method_desc = service_desc
        .methods()
        .find(|m| m.name() == method_name)
        .ok_or_else(|| format!("Method not found: '{}'", method_name))?;

    if !method_desc.is_server_streaming() {
        return Err("Method is not server streaming".to_string());
    }

    let input_desc = method_desc.input();

    let request_msg = DynamicMessage::deserialize(
        input_desc,
        &mut serde_json::Deserializer::from_str(message_json),
    )
    .map_err(|e| format!("JSON parse error: {}", e))?;
    let request_bytes = request_msg.encode_to_vec();
    let grpc_body = grpc_encode_message(&request_bytes);

    let http_url = url
        .trim_end_matches('/')
        .replace("grpc://", "http://")
        .replace("grpcs://", "https://");
    let full_url = format!("{}/{}", http_url, method_path);
    let use_h2c = http_url.starts_with("http://");

    let mut builder = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10));
    if use_h2c {
        builder = builder.http2_prior_knowledge();
    }
    let client = builder
        .build()
        .map_err(|e| format!("Client build error: {}", e))?;

    let mut req_builder = client
        .post(&full_url)
        .header("content-type", "application/grpc+proto")
        .header("te", "trailers");

    for (key, value) in &metadata {
        req_builder = req_builder.header(key.as_str(), value.as_str());
    }

    let response = req_builder
        .body(grpc_body)
        .send()
        .await
        .map_err(|e| format!("gRPC request failed: {}", e))?;

    let grpc_status = response
        .headers()
        .get("grpc-status")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(0);

    if grpc_status != 0 {
        let grpc_message = response
            .headers()
            .get("grpc-message")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("Unknown gRPC error")
            .to_string();
        return Err(format!("gRPC error status {}: {}", grpc_status, grpc_message));
    }

    let mut chunks = Vec::new();
    let mut buffer = Vec::new();
    let mut stream = response.bytes_stream();

    use futures_util::StreamExt;
    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(|e| format!("Read error: {}", e))?;
        buffer.extend_from_slice(&chunk);

        while buffer.len() >= 5 {
            let msg_len = u32::from_be_bytes([buffer[1], buffer[2], buffer[3], buffer[4]]) as usize;
            if buffer.len() >= 5 + msg_len {
                let msg_bytes = buffer.drain(5..5 + msg_len).collect::<Vec<_>>();
                if let Ok(response_msg) = DynamicMessage::decode(method_desc.output(), msg_bytes.as_ref()) {
                    if let Ok(json) = serde_json::to_string_pretty(&response_msg) {
                        chunks.push(json);
                    }
                }
            } else {
                break;
            }
        }
    }

    Ok(chunks)
}

/// Execute client streaming gRPC.
/// Sends multiple messages and returns a single response.
pub async fn execute_client_streaming(
    url: &str,
    method_full_name: &str,
    messages: Vec<String>,
    metadata: Vec<(String, String)>,
    proto_path: &Path,
) -> Result<String, String> {
    if messages.is_empty() {
        return Err("No messages to send".to_string());
    }

    let pool = parse_proto_file(proto_path)?;

    let method_path = method_full_name.trim_start_matches('/');
    let slash_pos = method_path
        .rfind('/')
        .ok_or_else(|| format!("Invalid method name '{}': missing '/'", method_full_name))?;
    let service_name = &method_path[..slash_pos];
    let method_name = &method_path[slash_pos + 1..];

    let service_desc = pool
        .get_service_by_name(service_name)
        .ok_or_else(|| format!("Service not found: '{}'", service_name))?;

    let method_desc = service_desc
        .methods()
        .find(|m| m.name() == method_name)
        .ok_or_else(|| format!("Method not found: '{}'", method_name))?;

    if !method_desc.is_client_streaming() {
        return Err("Method is not client streaming".to_string());
    }

    let output_desc = method_desc.output();

    let http_url = url
        .trim_end_matches('/')
        .replace("grpc://", "http://")
        .replace("grpcs://", "https://");
    let full_url = format!("{}/{}", http_url, method_path);
    let use_h2c = http_url.starts_with("http://");

    let mut builder = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10));
    if use_h2c {
        builder = builder.http2_prior_knowledge();
    }
    let client = builder
        .build()
        .map_err(|e| format!("Client build error: {}", e))?;

    let mut req_builder = client
        .post(&full_url)
        .header("content-type", "application/grpc+proto")
        .header("te", "trailers");

    for (key, value) in &metadata {
        req_builder = req_builder.header(key.as_str(), value.as_str());
    }

    let mut body = Vec::new();
    for msg_json in &messages {
        let inp_desc = method_desc.input();
        let request_msg = DynamicMessage::deserialize(
            inp_desc,
            &mut serde_json::Deserializer::from_str(msg_json),
        )
        .map_err(|e| format!("JSON parse error: {}", e))?;
        let request_bytes = request_msg.encode_to_vec();
        body.extend_from_slice(&grpc_encode_message(&request_bytes));
    }

    let response = req_builder
        .body(body)
        .send()
        .await
        .map_err(|e| format!("gRPC request failed: {}", e))?;

    let grpc_status = response
        .headers()
        .get("grpc-status")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(0);

    if grpc_status != 0 {
        let grpc_message = response
            .headers()
            .get("grpc-message")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("Unknown gRPC error")
            .to_string();
        return Err(format!("gRPC error status {}: {}", grpc_status, grpc_message));
    }

    let body_bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    let msg_bytes = grpc_decode_message(&body_bytes)?;

    let response_msg = DynamicMessage::decode(output_desc, msg_bytes.as_ref())
        .map_err(|e| format!("Protobuf decode error: {}", e))?;

    let response_json = serde_json::to_string_pretty(&response_msg)
        .map_err(|e| format!("JSON serialize error: {}", e))?;

    Ok(response_json)
}

/// Execute bidirectional streaming gRPC.
/// Note: Full bidirectional streaming requires WebSocket or HTTP/2 upgrade.
/// This implementation simulates it by sending all messages and collecting responses.
pub async fn execute_bidi_streaming(
    url: &str,
    method_full_name: &str,
    messages: Vec<String>,
    metadata: Vec<(String, String)>,
    proto_path: &Path,
) -> Result<Vec<String>, String> {
    if messages.is_empty() {
        return Err("No messages to send".to_string());
    }

    let pool = parse_proto_file(proto_path)?;

    let method_path = method_full_name.trim_start_matches('/');
    let slash_pos = method_path
        .rfind('/')
        .ok_or_else(|| format!("Invalid method name '{}': missing '/'", method_full_name))?;
    let service_name = &method_path[..slash_pos];
    let method_name = &method_path[slash_pos + 1..];

    let service_desc = pool
        .get_service_by_name(service_name)
        .ok_or_else(|| format!("Service not found: '{}'", service_name))?;

    let method_desc = service_desc
        .methods()
        .find(|m| m.name() == method_name)
        .ok_or_else(|| format!("Method not found: '{}'", method_name))?;

    if !method_desc.is_server_streaming() || !method_desc.is_client_streaming() {
        return Err("Method is not bidirectional streaming".to_string());
    }

    let http_url = url
        .trim_end_matches('/')
        .replace("grpc://", "http://")
        .replace("grpcs://", "https://");
    let full_url = format!("{}/{}", http_url, method_path);
    let use_h2c = http_url.starts_with("http://");

    let mut builder = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10));
    if use_h2c {
        builder = builder.http2_prior_knowledge();
    }
    let client = builder
        .build()
        .map_err(|e| format!("Client build error: {}", e))?;

    let mut req_builder = client
        .post(&full_url)
        .header("content-type", "application/grpc+proto")
        .header("te", "trailers");

    for (key, value) in &metadata {
        req_builder = req_builder.header(key.as_str(), value.as_str());
    }

    let mut body = Vec::new();
    for msg_json in &messages {
        let inp_desc = method_desc.input();
        let request_msg = DynamicMessage::deserialize(
            inp_desc,
            &mut serde_json::Deserializer::from_str(msg_json),
        )
        .map_err(|e| format!("JSON parse error: {}", e))?;
        let request_bytes = request_msg.encode_to_vec();
        body.extend_from_slice(&grpc_encode_message(&request_bytes));
    }

    let response = req_builder
        .body(body)
        .send()
        .await
        .map_err(|e| format!("gRPC request failed: {}", e))?;

    let grpc_status = response
        .headers()
        .get("grpc-status")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(0);

    if grpc_status != 0 {
        let grpc_message = response
            .headers()
            .get("grpc-message")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("Unknown gRPC error")
            .to_string();
        return Err(format!("gRPC error status {}: {}", grpc_status, grpc_message));
    }

    let mut chunks = Vec::new();
    let mut buffer = Vec::new();
    let mut stream = response.bytes_stream();

    use futures_util::StreamExt;
    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(|e| format!("Read error: {}", e))?;
        buffer.extend_from_slice(&chunk);

        while buffer.len() >= 5 {
            let msg_len = u32::from_be_bytes([buffer[1], buffer[2], buffer[3], buffer[4]]) as usize;
            if buffer.len() >= 5 + msg_len {
                let msg_bytes = buffer.drain(5..5 + msg_len).collect::<Vec<_>>();
                if let Ok(response_msg) = DynamicMessage::decode(method_desc.output(), msg_bytes.as_ref()) {
                    if let Ok(json) = serde_json::to_string_pretty(&response_msg) {
                        chunks.push(json);
                    }
                }
            } else {
                break;
            }
        }
    }

    Ok(chunks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grpc_encode_decode_roundtrip() {
        let msg = b"hello world";
        let encoded = grpc_encode_message(msg);
        assert_eq!(encoded[0], 0);
        assert_eq!(
            u32::from_be_bytes([encoded[1], encoded[2], encoded[3], encoded[4]]),
            msg.len() as u32
        );
        let decoded = grpc_decode_message(&encoded).unwrap();
        assert_eq!(decoded, msg);
    }

    #[test]
    fn test_grpc_decode_too_short() {
        assert!(grpc_decode_message(&[0, 0, 0]).is_err());
    }
}
