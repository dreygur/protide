//! gRPC message encoding/decoding and shared descriptor helpers

use prost_reflect::DescriptorPool;

pub(super) fn grpc_encode_message(msg_bytes: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(5 + msg_bytes.len());
    buf.push(0u8); // compression flag: not compressed
    buf.extend_from_slice(&(msg_bytes.len() as u32).to_be_bytes());
    buf.extend_from_slice(msg_bytes);
    buf
}

pub(super) fn grpc_decode_message(data: &[u8]) -> Result<Vec<u8>, String> {
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
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct StreamingChunk {
    pub data: String,
    pub is_final: bool,
    pub is_error: bool,
}

#[allow(dead_code)]
impl StreamingChunk {
    pub(super) fn error(msg: String) -> Self {
        Self {
            data: msg,
            is_final: true,
            is_error: true,
        }
    }

    pub(super) fn data(data: String, is_final: bool) -> Self {
        Self {
            data,
            is_final,
            is_error: false,
        }
    }
}

/// Resolve a `MethodDescriptor` from `pool` given `"[pkg.]Service/Method"`.
pub(super) fn resolve_method(
    pool: &DescriptorPool,
    method_full_name: &str,
) -> Result<prost_reflect::MethodDescriptor, String> {
    let method_path = method_full_name.trim_start_matches('/');
    let slash_pos = method_path
        .rfind('/')
        .ok_or_else(|| format!("Invalid method name '{}': missing '/'", method_full_name))?;
    let service_name = &method_path[..slash_pos];
    let method_name = &method_path[slash_pos + 1..];

    let service_desc = pool
        .get_service_by_name(service_name)
        .ok_or_else(|| format!("Service not found: '{}'", service_name))?;

    service_desc
        .methods()
        .find(|m| m.name() == method_name)
        .ok_or_else(|| format!("Method not found: '{}'", method_name))
}
