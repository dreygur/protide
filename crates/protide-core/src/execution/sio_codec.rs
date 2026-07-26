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
///
/// A blank `payload` encodes an event with no argument (`["name"]`); emitting
/// `["name",]` instead would be malformed JSON and servers drop the connection.
pub(super) fn encode_sio_event(
    namespace: &str,
    event_name: &str,
    payload: &str,
    ack_id: Option<u32>,
) -> String {
    let escaped_name = event_name.replace('\\', "\\\\").replace('"', "\\\"");
    let data = if payload.trim().is_empty() {
        format!("[\"{}\"]", escaped_name)
    } else {
        format!("[\"{}\",{}]", escaped_name, payload)
    };
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
    let digit_end = after_ns
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(after_ns.len());
    let (ack_id, data) = if digit_end > 0 && !after_ns.starts_with('[') {
        (
            after_ns[..digit_end].parse::<u32>().ok(),
            &after_ns[digit_end..],
        )
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Inputs a hostile or buggy server could put on the wire. No codec
    /// function may panic on any of them - a panic in the reader task would
    /// take down the whole app, not just the connection.
    const ADVERSARIAL: &[&str] = &[
        "",
        "4",
        "42",
        "2",
        "2/",
        "2/,",
        "2/admin",
        "2/admin,",
        "2/admin,7",
        "42[",
        "42[\"unterminated",
        "2[\"a\",{\"b\":]",
        "9999999999999999999999[\"x\"]",
        "2/admin,99999999999999999999[\"x\"]",
        "\u{1f600}",
        "2\u{1f600}",
        "4\u{feff}0",
        "2/日本語,[\"イベント\",{\"k\":\"値\"}]",
        "٤2[\"x\"]",
        "-1[\"x\"]",
        "2[]",
        "2[null]",
        "2[123]",
        "2{\"not\":\"an array\"}",
        "\n\r\0",
    ];

    #[test]
    fn codec_never_panics_on_adversarial_input() {
        for raw in ADVERSARIAL {
            let _ = parse_eio_type(raw);
            let _ = parse_sio_header(raw);
            let _ = parse_event_array(raw);
            let _ = build_ws_url(raw);
            let _ = encode_sio_event(raw, raw, raw, Some(7));
            let _ = encode_sio_connect(raw);
        }
    }

    /// Truncating a valid frame at every byte boundary must never panic and
    /// must never mis-slice a multi-byte character.
    #[test]
    fn parsing_truncated_frames_never_panics() {
        let full = "42/日本,7[\"イベント\",{\"k\":\"値\"}]";
        for end in 0..=full.len() {
            if !full.is_char_boundary(end) {
                continue;
            }
            let partial = &full[..end];
            let _ = parse_eio_type(partial);
            let _ = parse_sio_header(partial);
        }
    }

    // ── build_ws_url ─────────────────────────────────────────────────────────

    #[test]
    fn build_ws_url_keeps_path_and_appends_engineio_query() {
        assert_eq!(
            build_ws_url("http://localhost:3000/base"),
            "ws://localhost:3000/base/socket.io/?EIO=4&transport=websocket"
        );
    }

    #[test]
    fn build_ws_url_leaves_explicit_socketio_path_alone() {
        let explicit = "wss://example.com/socket.io/?EIO=4&transport=websocket";
        assert_eq!(build_ws_url(explicit), explicit);
    }

    // ── encode ───────────────────────────────────────────────────────────────

    #[test]
    fn encode_connect_default_and_custom_namespace() {
        assert_eq!(encode_sio_connect("/"), "0");
        assert_eq!(encode_sio_connect("/admin"), "0/admin,");
    }

    /// The event name is embedded in a hand-built JSON string, so quotes and
    /// backslashes must be escaped or the frame is unparsable.
    #[test]
    fn encode_event_escapes_name() {
        let frame = encode_sio_event("/", r#"we"ird\name"#, "1", None);
        assert_eq!(frame, r#"2["we\"ird\\name",1]"#);
        assert!(serde_json::from_str::<serde_json::Value>(&frame[1..]).is_ok());
    }

    /// An empty payload (the user left the payload box blank) must produce a
    /// zero-argument event, not the malformed `["evt",]`.
    #[test]
    fn encode_event_with_blank_payload_is_valid_json() {
        for blank in ["", "   ", "\n"] {
            let frame = encode_sio_event("/", "evt", blank, None);
            assert_eq!(frame, r#"2["evt"]"#);
            let parsed: serde_json::Value =
                serde_json::from_str(&frame[1..]).expect("frame data must be valid JSON");
            assert_eq!(parsed, serde_json::json!(["evt"]));
        }
    }

    #[test]
    fn encode_event_with_blank_payload_keeps_namespace_and_ack() {
        assert_eq!(
            encode_sio_event("/admin", "evt", "", Some(3)),
            r#"2/admin,3["evt"]"#
        );
    }

    /// A payload that is not valid JSON is spliced into the frame unchanged,
    /// producing a malformed packet (`2["evt",hello]`) that a Socket.IO server
    /// rejects, usually by closing the connection with no visible error.
    /// Deliberately unfixed: whether a bare payload should be quoted as a JSON
    /// string, rejected up-front in the UI, or sent as-is is a product
    /// decision, not something to change under a test.
    #[test]
    #[ignore = "known defect: non-JSON payloads produce a malformed frame (see comment)"]
    fn encode_event_with_non_json_payload_should_not_produce_malformed_frame() {
        let frame = encode_sio_event("/", "evt", "hello", None);
        serde_json::from_str::<serde_json::Value>(&frame[1..])
            .expect("frame data must be valid JSON");
    }

    // ── round-trip ───────────────────────────────────────────────────────────

    #[test]
    fn event_round_trips_through_header_and_array_parsers() {
        let cases: [(&str, &str, &str, Option<u32>); 4] = [
            ("/", "chat", r#"{"msg":"hi"}"#, None),
            ("/admin", "kick", r#""user1""#, Some(5)),
            ("/", "unicode", r#"{"t":"héllo — 日本 🎉"}"#, Some(0)),
            ("/ns", "with,comma", r#"[1,2,3]"#, Some(4_294_967_295)),
        ];
        for (ns, name, payload, ack) in cases {
            let frame = encode_sio_event(ns, name, payload, ack);
            let (ptype, parsed_ns, parsed_ack, data) =
                parse_sio_header(&frame).expect("header parses");
            assert_eq!(ptype, 2, "frame: {frame}");
            assert_eq!(parsed_ns, ns, "frame: {frame}");
            assert_eq!(parsed_ack, ack, "frame: {frame}");

            let (parsed_name, parsed_payload) =
                parse_event_array(data).expect("event array parses");
            assert_eq!(parsed_name, name, "frame: {frame}");
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(&parsed_payload).expect("payload JSON"),
                serde_json::from_str::<serde_json::Value>(payload).expect("payload JSON"),
                "frame: {frame}"
            );
        }
    }

    // ── parse_eio_type ───────────────────────────────────────────────────────

    #[test]
    fn parse_eio_type_splits_first_char() {
        assert_eq!(parse_eio_type("4hello"), Some(('4', "hello")));
        assert_eq!(parse_eio_type("2"), Some(('2', "")));
        assert_eq!(parse_eio_type(""), None);
    }

    /// The split must be by character, not byte, or a multi-byte leading
    /// character would slice mid-codepoint.
    #[test]
    fn parse_eio_type_handles_multibyte_leading_char() {
        assert_eq!(parse_eio_type("é4"), Some(('é', "4")));
    }

    // ── parse_sio_header ─────────────────────────────────────────────────────

    #[test]
    fn parse_sio_header_rejects_non_digit_packet_type() {
        assert_eq!(parse_sio_header(""), None);
        assert_eq!(parse_sio_header("x[\"a\"]"), None);
        assert_eq!(parse_sio_header("٤[\"a\"]"), None);
    }

    #[test]
    fn parse_sio_header_disconnect_namespace_without_comma() {
        let (ptype, ns, ack, data) = parse_sio_header("1/admin").expect("parses");
        assert_eq!((ptype, ns.as_str(), ack, data), (1, "/admin", None, ""));
    }

    #[test]
    fn parse_sio_header_connect_payload_is_not_read_as_ack() {
        let (ptype, ns, ack, data) = parse_sio_header(r#"0{"sid":"abc"}"#).expect("parses");
        assert_eq!(ptype, 0);
        assert_eq!(ns, "/");
        assert_eq!(ack, None);
        assert_eq!(data, r#"{"sid":"abc"}"#);
    }

    /// An ack id too large for `u32` must degrade to "no ack id" rather than
    /// panicking or swallowing the payload.
    #[test]
    fn parse_sio_header_overflowing_ack_id_is_dropped_not_fatal() {
        let (ptype, ns, ack, data) =
            parse_sio_header("299999999999999999999[\"x\"]").expect("parses");
        assert_eq!(ptype, 2);
        assert_eq!(ns, "/");
        assert_eq!(ack, None);
        assert_eq!(data, "[\"x\"]");
    }

    // ── parse_event_array ────────────────────────────────────────────────────

    #[test]
    fn parse_event_array_rejects_malformed_and_wrong_shapes() {
        assert_eq!(parse_event_array(""), None);
        assert_eq!(parse_event_array("["), None);
        assert_eq!(parse_event_array("[]"), None);
        assert_eq!(
            parse_event_array("[123]"),
            None,
            "event name must be a string"
        );
        assert_eq!(parse_event_array("[null,1]"), None);
        assert_eq!(parse_event_array(r#"{"a":1}"#), None, "not an array");
    }

    /// Only the first argument is surfaced today; extra arguments are dropped
    /// rather than causing a parse failure.
    #[test]
    fn parse_event_array_takes_first_argument_only() {
        let (name, payload) = parse_event_array(r#"["evt",{"a":1},"extra"]"#).expect("parses");
        assert_eq!(name, "evt");
        assert_eq!(payload, r#"{"a":1}"#);
    }

    #[test]
    fn parse_event_array_preserves_unicode() {
        let (name, payload) = parse_event_array(r#"["イベント","héllo 🎉"]"#).expect("parses");
        assert_eq!(name, "イベント");
        assert_eq!(payload, r#""héllo 🎉""#);
    }
}
