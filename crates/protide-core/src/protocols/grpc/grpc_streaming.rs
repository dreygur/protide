//! Async gRPC streaming execution (server, client, bidi)

use super::grpc_encoding::{grpc_decode_message, grpc_encode_message, resolve_method};
use super::parse_proto_file;
use futures_util::StreamExt;
use prost::Message;
use prost_reflect::DynamicMessage;
use std::path::Path;
use std::time::Duration;

/// Result of draining a streaming gRPC response.
///
/// `dropped_frames` counts frames whose length prefix was well-formed but that
/// failed to decode (e.g. corrupt bytes, or a compressed frame that failed to
/// decompress) — those frames are excluded from `chunks` rather than silently
/// vanishing from the result entirely.
#[derive(Debug, Clone, Default)]
pub struct StreamingResult {
    pub chunks: Vec<String>,
    pub dropped_frames: usize,
}

/// Execute server streaming gRPC using async HTTP/2.
/// Returns the decoded response chunks (JSON strings) plus a count of any
/// frames that could not be decoded.
pub async fn execute_server_streaming(
    url: &str,
    method_full_name: &str,
    message_json: &str,
    metadata: Vec<(String, String)>,
    proto_path: &Path,
) -> Result<StreamingResult, String> {
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
    let mut dropped_frames = 0usize;
    let mut buffer = Vec::new();
    let mut stream = response.bytes_stream();

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(|e| format!("Read error: {}", e))?;
        buffer.extend_from_slice(&chunk);
        drain_frames(&mut buffer, &method_desc, &mut chunks, &mut dropped_frames);
    }

    Ok(StreamingResult {
        chunks,
        dropped_frames,
    })
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
) -> Result<StreamingResult, String> {
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
    let mut dropped_frames = 0usize;
    let mut buffer = Vec::new();
    let mut stream = response.bytes_stream();

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(|e| format!("Read error: {}", e))?;
        buffer.extend_from_slice(&chunk);
        drain_frames(&mut buffer, &method_desc, &mut chunks, &mut dropped_frames);
    }

    Ok(StreamingResult {
        chunks,
        dropped_frames,
    })
}

// --- helpers ----------------------------------------------------------------

fn build_async_client(url: &str, method_path: &str) -> Result<(reqwest::Client, String), String> {
    let http_url = url
        .trim_end_matches('/')
        .replace("grpc://", "http://")
        .replace("grpcs://", "https://");
    let full_url = format!("{}/{}", http_url, method_path);
    let use_h2c = http_url.starts_with("http://");

    let mut builder = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30));
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
        return Err(format!(
            "gRPC error status {}: {}",
            grpc_status, grpc_message
        ));
    }
    Ok(())
}

fn drain_frames(
    buffer: &mut Vec<u8>,
    method_desc: &prost_reflect::MethodDescriptor,
    chunks: &mut Vec<String>,
    dropped_frames: &mut usize,
) {
    while buffer.len() >= 5 {
        let msg_len = u32::from_be_bytes([buffer[1], buffer[2], buffer[3], buffer[4]]) as usize;
        if buffer.len() < 5 + msg_len {
            break;
        }
        let frame = buffer.drain(..5 + msg_len).collect::<Vec<_>>();
        let decoded = grpc_decode_message(&frame).and_then(|msg_bytes| {
            DynamicMessage::decode(method_desc.output(), msg_bytes.as_ref())
                .map_err(|e| format!("Protobuf decode error: {}", e))
        });
        match decoded {
            Ok(response_msg) => match serde_json::to_string_pretty(&response_msg) {
                Ok(json) => chunks.push(json),
                Err(e) => {
                    log::warn!("gRPC stream frame dropped (JSON serialize error): {}", e);
                    *dropped_frames += 1;
                }
            },
            Err(e) => {
                log::warn!("gRPC stream frame dropped (decode error): {}", e);
                *dropped_frames += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    /// Writes a minimal .proto file to a uniquely-named temp dir and returns
    /// the resolved `MethodDescriptor` for `test.TestService/Stream`, whose
    /// reply type has a single `string msg = 1` field. The backing directory is
    /// deleted when the returned guard is dropped.
    fn test_method_desc() -> (TempDir, prost_reflect::MethodDescriptor) {
        let dir = TempDir::new("protide_grpc_streaming_test");
        let proto_path = dir.write(
            "test.proto",
            br#"
            syntax = "proto3";
            package test;
            message Empty {}
            message Reply { string msg = 1; }
            service TestService {
              rpc Stream(Empty) returns (stream Reply);
            }
            "#,
        );
        let pool = parse_proto_file(&proto_path).unwrap();
        let method_desc = resolve_method(&pool, "test.TestService/Stream").unwrap();
        (dir, method_desc)
    }

    fn encode_reply(method_desc: &prost_reflect::MethodDescriptor, msg: &str) -> Vec<u8> {
        let json = format!(r#"{{"msg":"{}"}}"#, msg);
        let dynamic = DynamicMessage::deserialize(
            method_desc.output(),
            &mut serde_json::Deserializer::from_str(&json),
        )
        .unwrap();
        dynamic.encode_to_vec()
    }

    #[test]
    fn drain_frames_surfaces_decode_failures_instead_of_silently_dropping() {
        let (_dir, method_desc) = test_method_desc();

        let mut buffer = Vec::new();
        // Frame 1: valid, uncompressed.
        buffer.extend_from_slice(&grpc_encode_message(&encode_reply(&method_desc, "hello")));
        // Frame 2: corrupt — claims compression but payload isn't valid gzip,
        // so it must fail to decode rather than vanish silently.
        let garbage = b"not gzip data".to_vec();
        buffer.push(1u8); // compression flag set
        buffer.extend_from_slice(&(garbage.len() as u32).to_be_bytes());
        buffer.extend_from_slice(&garbage);
        // Frame 3: valid, uncompressed.
        buffer.extend_from_slice(&grpc_encode_message(&encode_reply(&method_desc, "world")));

        let mut chunks = Vec::new();
        let mut dropped_frames = 0usize;
        drain_frames(&mut buffer, &method_desc, &mut chunks, &mut dropped_frames);

        // Before the fix, the corrupt frame was silently dropped with no
        // signal at all. Now it must be counted...
        assert_eq!(
            dropped_frames, 1,
            "the corrupt frame must be counted as dropped"
        );
        // ...while the two good frames on either side still decode fine.
        assert_eq!(chunks.len(), 2);
        assert!(chunks[0].contains("hello"));
        assert!(chunks[1].contains("world"));
        assert!(
            buffer.is_empty(),
            "all well-framed bytes should be consumed"
        );
    }

    #[test]
    fn drain_frames_decompresses_gzip_compressed_frames() {
        use flate2::Compression;
        use flate2::write::GzEncoder;
        use std::io::Write;

        let (_dir, method_desc) = test_method_desc();
        let raw = encode_reply(&method_desc, "compressed-hello");

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&raw).unwrap();
        let compressed = encoder.finish().unwrap();

        let mut buffer = Vec::new();
        buffer.push(1u8); // compression flag set
        buffer.extend_from_slice(&(compressed.len() as u32).to_be_bytes());
        buffer.extend_from_slice(&compressed);

        let mut chunks = Vec::new();
        let mut dropped_frames = 0usize;
        drain_frames(&mut buffer, &method_desc, &mut chunks, &mut dropped_frames);

        assert_eq!(dropped_frames, 0);
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].contains("compressed-hello"));
    }
}
