/// Convert a base URL (http/https) to the Engine.IO WebSocket endpoint.
pub(super) fn build_ws_url(base_url: &str) -> String {
    let base = base_url.trim_end_matches('/');
    // Convert scheme: http → ws, https → wss
    let ws_base = if base.starts_with("https://") {
        base.replacen("https://", "wss://", 1)
    } else {
        base.replacen("http://", "ws://", 1)
    };
    // Append Socket.IO path unless already present
    if ws_base.contains("/socket.io") {
        ws_base
    } else {
        format!("{}/socket.io/?EIO=4&transport=websocket", ws_base)
    }
}

/// Encode a Socket.IO CONNECT packet for a namespace.
pub(super) fn encode_sio_connect(namespace: &str) -> String {
    if namespace == "/" {
        "0".to_string()
    } else {
        format!("0{},", namespace)
    }
}

/// Encode a Socket.IO EVENT packet (type 2).
pub(super) fn encode_sio_event(namespace: &str, event_name: &str, payload: &str, ack_id: Option<u32>) -> String {
    let escaped_name = event_name.replace('\\', "\\\\").replace('"', "\\\"");
    let data = format!("[\"{}\",{}]", escaped_name, payload);
    let ack_str = ack_id.map(|id| id.to_string()).unwrap_or_default();
    if namespace == "/" {
        format!("2{}{}", ack_str, data)
    } else {
        format!("2{},{}{}", namespace, ack_str, data)
    }
}

/// Return (eio_type_char, rest) for an Engine.IO raw message.
pub(super) fn parse_eio_type(raw: &str) -> Option<(char, &str)> {
    let c = raw.chars().next()?;
    Some((c, &raw[c.len_utf8()..]))
}

/// Parse a Socket.IO packet. Returns (ptype, namespace, ack_id, data_slice).
pub(super) fn parse_sio_header(raw: &str) -> Option<(u8, String, Option<u32>, &str)> {
    let ptype = raw.chars().next()?.to_digit(10)? as u8;
    let rest = &raw[1..];

    let (namespace, after_ns) = if rest.starts_with('/') {
        // Namespace ends at ',' or (for DISCONNECT) at end of string
        if let Some(pos) = rest.find(',') {
            (rest[..pos].to_string(), &rest[pos + 1..])
        } else {
            (rest.to_string(), "")
        }
    } else {
        ("/".to_string(), rest)
    };

    // Optional ack id: leading digits before the first non-digit (e.g. '[')
    let digit_end = after_ns.find(|c: char| !c.is_ascii_digit()).unwrap_or(after_ns.len());
    let (ack_id, data) = if digit_end > 0 && !after_ns.starts_with('[') {
        (after_ns[..digit_end].parse::<u32>().ok(), &after_ns[digit_end..])
    } else {
        (None, after_ns)
    };

    Some((ptype, namespace, ack_id, data))
}

/// Extract (event_name, payload) from a Socket.IO event JSON array.
pub(super) fn parse_event_array(json: &str) -> Option<(String, String)> {
    let arr: serde_json::Value = serde_json::from_str(json).ok()?;
    let arr = arr.as_array()?;
    let name = arr.first()?.as_str()?.to_string();
    let payload = if arr.len() > 1 {
        serde_json::to_string(&arr[1]).unwrap_or_else(|_| "null".into())
    } else {
        "null".to_string()
    };
    Some((name, payload))
}
