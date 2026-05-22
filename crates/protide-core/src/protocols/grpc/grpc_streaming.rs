//! Async gRPC streaming execution (server, client, bidi)

use super::grpc_encoding::{grpc_decode_message, grpc_encode_message, resolve_method};
use super::parse_proto_file;
use futures_util::StreamExt;
use prost::Message;
use prost_reflect::DynamicMessage;
use std::path::Path;
use std::time::Duration;

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
    let method_desc = resolve_method(&pool, method_full_name)?;

    if !method_desc.is_server_streaming() {
        return Err("Method is not server streaming".to_string());
    }

    let request_msg = DynamicMessage::deserialize(
        method_desc.input(),
        &mut serde_json::Deserializer::from_str(message_json),
    )
    .map_err(|e| format!("JSON parse error: {}", e))?;
    let grpc_body = grpc_encode_message(&request_msg.encode_to_vec());

    let method_path = method_full_name.trim_start_matches('/');
    let (client, full_url) = build_async_client(url, method_path)?;

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

    check_grpc_status(&response)?;

    let mut chunks = Vec::new();
    let mut buffer = Vec::new();
    let mut stream = response.bytes_stream();

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(|e| format!("Read error: {}", e))?;
        buffer.extend_from_slice(&chunk);
        drain_frames(&mut buffer, &method_desc, &mut chunks);
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
    let method_desc = resolve_method(&pool, method_full_name)?;

    if !method_desc.is_client_streaming() {
        return Err("Method is not client streaming".to_string());
    }

    let method_path = method_full_name.trim_start_matches('/');
    let (client, full_url) = build_async_client(url, method_path)?;

    let mut req_builder = client
        .post(&full_url)
        .header("content-type", "application/grpc+proto")
        .header("te", "trailers");
    for (key, value) in &metadata {
        req_builder = req_builder.header(key.as_str(), value.as_str());
    }

    let mut body = Vec::new();
    for msg_json in &messages {
        let request_msg = DynamicMessage::deserialize(
            method_desc.input(),
            &mut serde_json::Deserializer::from_str(msg_json),
        )
        .map_err(|e| format!("JSON parse error: {}", e))?;
        body.extend_from_slice(&grpc_encode_message(&request_msg.encode_to_vec()));
    }

    let response = req_builder
        .body(body)
        .send()
        .await
        .map_err(|e| format!("gRPC request failed: {}", e))?;

    check_grpc_status(&response)?;

    let body_bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    let msg_bytes = grpc_decode_message(&body_bytes)?;
    let response_msg = DynamicMessage::decode(method_desc.output(), msg_bytes.as_ref())
        .map_err(|e| format!("Protobuf decode error: {}", e))?;
    serde_json::to_string_pretty(&response_msg).map_err(|e| format!("JSON serialize error: {}", e))
}

/// Execute bidirectional streaming gRPC.
/// Simulates bidi by sending all messages then collecting all responses.
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
    let method_desc = resolve_method(&pool, method_full_name)?;

    if !method_desc.is_server_streaming() || !method_desc.is_client_streaming() {
        return Err("Method is not bidirectional streaming".to_string());
    }

    let method_path = method_full_name.trim_start_matches('/');
    let (client, full_url) = build_async_client(url, method_path)?;

    let mut req_builder = client
        .post(&full_url)
        .header("content-type", "application/grpc+proto")
        .header("te", "trailers");
    for (key, value) in &metadata {
        req_builder = req_builder.header(key.as_str(), value.as_str());
    }

    let mut body = Vec::new();
    for msg_json in &messages {
        let request_msg = DynamicMessage::deserialize(
            method_desc.input(),
            &mut serde_json::Deserializer::from_str(msg_json),
        )
        .map_err(|e| format!("JSON parse error: {}", e))?;
        body.extend_from_slice(&grpc_encode_message(&request_msg.encode_to_vec()));
    }

    let response = req_builder
        .body(body)
        .send()
        .await
        .map_err(|e| format!("gRPC request failed: {}", e))?;

    check_grpc_status(&response)?;

    let mut chunks = Vec::new();
    let mut buffer = Vec::new();
    let mut stream = response.bytes_stream();

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(|e| format!("Read error: {}", e))?;
        buffer.extend_from_slice(&chunk);
        drain_frames(&mut buffer, &method_desc, &mut chunks);
    }

    Ok(chunks)
}

// --- helpers ----------------------------------------------------------------

fn build_async_client(url: &str, method_path: &str) -> Result<(reqwest::Client, String), String> {
    let http_url = url
        .trim_end_matches('/')
        .replace("grpc://", "http://")
        .replace("grpcs://", "https://");
    let full_url = format!("{}/{}", http_url, method_path);
    let use_h2c = http_url.starts_with("http://");

    let mut builder = reqwest::Client::builder().connect_timeout(Duration::from_secs(10));
    if use_h2c {
        builder = builder.http2_prior_knowledge();
    }
    let client = builder
        .build()
        .map_err(|e| format!("Client build error: {}", e))?;
    Ok((client, full_url))
}

fn check_grpc_status(response: &reqwest::Response) -> Result<(), String> {
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
    Ok(())
}

fn drain_frames(
    buffer: &mut Vec<u8>,
    method_desc: &prost_reflect::MethodDescriptor,
    chunks: &mut Vec<String>,
) {
    while buffer.len() >= 5 {
        let msg_len =
            u32::from_be_bytes([buffer[1], buffer[2], buffer[3], buffer[4]]) as usize;
        if buffer.len() >= 5 + msg_len {
            let msg_bytes = buffer.drain(..5 + msg_len).skip(5).collect::<Vec<_>>();
            if let Ok(response_msg) =
                DynamicMessage::decode(method_desc.output(), msg_bytes.as_ref())
            {
                if let Ok(json) = serde_json::to_string_pretty(&response_msg) {
                    chunks.push(json);
                }
            }
        } else {
            break;
        }
    }
}
