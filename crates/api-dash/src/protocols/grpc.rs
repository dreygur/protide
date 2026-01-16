//! gRPC protocol support using prost-reflect for dynamic proto handling

use prost_reflect::{DescriptorPool, DynamicMessage, MethodDescriptor, ServiceDescriptor};
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

/// Parse a proto file content into a descriptor pool
///
/// This uses prost-reflect to parse proto files at runtime without compile-time codegen.
pub fn parse_proto_file(content: &str) -> Result<DescriptorPool, String> {
    // Try to parse as FileDescriptorSet
    // prost-reflect expects proto files to be compiled to FileDescriptorSet format
    // For raw .proto files, we need to use protoc or a parser

    // For now, return error with helpful message
    // In a full implementation, we would:
    // 1. Use protox or similar to parse .proto syntax into FileDescriptorProto
    // 2. Build DescriptorPool from FileDescriptorProto

    Err("Proto file parsing requires protoc integration. Use precompiled .pb files or integrate protox crate.".to_string())
}

/// Extract services and methods from a descriptor pool
pub fn extract_services(pool: &DescriptorPool) -> Vec<GrpcService> {
    let mut services = Vec::new();

    for service_desc in pool.services() {
        let mut methods = Vec::new();

        for method_desc in service_desc.methods() {
            methods.push(GrpcMethod {
                name: method_desc.name().to_string(),
                full_name: method_desc.full_name().to_string(),
                input_type: method_desc.input().full_name().to_string(),
                output_type: method_desc.output().full_name().to_string(),
                is_client_streaming: method_desc.is_client_streaming(),
                is_server_streaming: method_desc.is_server_streaming(),
            });
        }

        services.push(GrpcService {
            name: service_desc.name().to_string(),
            full_name: service_desc.full_name().to_string(),
            methods,
        });
    }

    services
}

/// Execute a unary gRPC call
///
/// # Arguments
/// * `url` - The gRPC server URL (e.g., "http://localhost:50051")
/// * `method_full_name` - Full method name (e.g., "/package.Service/Method")
/// * `message_json` - JSON representation of the message
/// * `metadata` - gRPC metadata (headers)
/// * `descriptor_pool` - Descriptor pool for message serialization
///
/// # Returns
/// Result containing (response_json, elapsed_time) or error string
pub async fn execute_unary(
    url: &str,
    method_full_name: &str,
    message_json: &str,
    metadata: Vec<(String, String)>,
    descriptor_pool: &DescriptorPool,
) -> Result<(String, Duration), String> {
    let start = std::time::Instant::now();

    // Get method descriptor
    let method_desc = descriptor_pool
        .get_service_by_name(&extract_service_name(method_full_name))
        .and_then(|svc| {
            svc.methods()
                .find(|m| m.full_name() == method_full_name || format!("/{}", m.full_name()) == method_full_name)
        })
        .ok_or_else(|| format!("Method not found: {}", method_full_name))?;

    // Parse JSON into DynamicMessage
    let _request_msg = json_to_dynamic_message(message_json, method_desc.input(), descriptor_pool)?;

    // Serialize to protobuf bytes
    // TODO: Implement proper encoding once prost-reflect API is correctly used
    let request_bytes: Vec<u8> = Vec::new(); // _request_msg.encode_to_vec();
    let request_size = request_bytes.len();

    // Create tonic channel
    let _channel = tonic::transport::Channel::from_shared(url.to_string())
        .map_err(|e| format!("Invalid URL: {}", e))?
        .connect()
        .await
        .map_err(|e| format!("Connection failed: {}", e))?;

    // Build request with metadata
    let mut request = tonic::Request::new(request_bytes);
    for (key, value) in metadata {
        if let (Ok(key_name), Ok(val)) = (
            tonic::metadata::MetadataKey::from_bytes(key.as_bytes()),
            tonic::metadata::MetadataValue::try_from(&value),
        ) {
            request.metadata_mut().insert(key_name, val);
        }
    }

    // Execute unary call using generic codec
    // Note: This requires tonic::codec::ProstCodec which works with bytes
    use tonic::codec::{Codec, DecodeBuf, EncodeBuf};

    // For simplicity, we'll use HTTP/2 directly
    // In a full implementation, use tonic::client::Grpc with dynamic codec

    // Placeholder: Return success with timing
    let elapsed = start.elapsed();

    let response_json = serde_json::json!({
        "note": "gRPC unary execution partially implemented",
        "method": method_full_name,
        "request_size": request_size,
        "elapsed_ms": elapsed.as_millis(),
        "status": "Connected to channel successfully",
        "next_steps": "Implement dynamic codec for prost-reflect DynamicMessage"
    });

    Ok((serde_json::to_string_pretty(&response_json).unwrap(), elapsed))
}

/// Convert JSON string to DynamicMessage
fn json_to_dynamic_message(
    json: &str,
    message_desc: prost_reflect::MessageDescriptor,
    _pool: &DescriptorPool,
) -> Result<DynamicMessage, String> {
    // Parse JSON
    let _json_value: serde_json::Value = serde_json::from_str(json)
        .map_err(|e| format!("Invalid JSON: {}", e))?;

    // Create dynamic message from JSON
    // TODO: Use correct prost-reflect API for JSON deserialization
    let msg = DynamicMessage::new(message_desc);
    // let msg = DynamicMessage::deserialize(message_desc, _json_value)
    //     .map_err(|e| format!("Failed to convert JSON to protobuf: {}", e))?;

    Ok(msg)
}

/// Convert DynamicMessage to JSON string
fn dynamic_message_to_json(_msg: &DynamicMessage) -> Result<String, String> {
    // TODO: Use correct prost-reflect API for JSON serialization
    // let json_value = _msg.serialize_to_json_value()
    //     .map_err(|e| format!("Failed to convert protobuf to JSON: {}", e))?;
    let json_value = serde_json::json!({"status": "todo"});

    serde_json::to_string_pretty(&json_value)
        .map_err(|e| format!("Failed to serialize JSON: {}", e))
}

/// Extract service name from full method name
/// E.g., "/package.Service/Method" -> "package.Service"
fn extract_service_name(full_method: &str) -> String {
    let trimmed = full_method.trim_start_matches('/');
    if let Some(slash_pos) = trimmed.rfind('/') {
        trimmed[..slash_pos].to_string()
    } else if let Some(dot_pos) = trimmed.rfind('.') {
        trimmed[..=dot_pos].trim_end_matches('.').to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_service_name() {
        assert_eq!(extract_service_name("/greet.Greeter/SayHello"), "greet.Greeter");
        assert_eq!(extract_service_name("greet.Greeter/SayHello"), "greet.Greeter");
        assert_eq!(extract_service_name("/Greeter/SayHello"), "Greeter");
    }
}
