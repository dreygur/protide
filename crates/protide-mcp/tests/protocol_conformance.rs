//! Regression tests proving the protide-mcp stdio server silently swallows
//! certain classes of syntactically-valid-or-invalid JSON-RPC 2.0 input
//! instead of returning a JSON-RPC error response, per the spec
//! (https://www.jsonrpc.org/specification#error_object -- Parse error -32700,
//! and batch-request handling in section 6).
//!
//! Both scenarios below cause the server to emit *zero* bytes on stdout for
//! a message that a spec-conformant client is entitled to expect a response
//! for, which means a caller waiting on that response (matched by `id`) will
//! hang forever / time out rather than receiving a clean error.

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::time::Duration;

/// Spawn the built protide-mcp binary, write `input` to its stdin, close
/// stdin, and collect whatever lines it wrote to stdout within a bounded
/// time (spawned in a helper thread so a hang doesn't hang the test suite).
fn run_mcp(input: &str) -> Vec<String> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_protide-mcp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn protide-mcp binary");

    let mut stdin = child.stdin.take().unwrap();
    let input = input.to_string();
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(input.as_bytes());
        // Dropping `stdin` here closes the pipe, which causes the server's
        // `stdin.lock().lines()` loop to terminate (EOF) and the process to exit.
    });

    let stdout = child.stdout.take().unwrap();
    let reader_handle = std::thread::spawn(move || {
        let reader = BufReader::new(stdout);
        reader.lines().map_while(Result::ok).collect::<Vec<String>>()
    });

    writer.join().unwrap();

    // Bound the wait so a genuine hang fails the test instead of blocking forever.
    let start = std::time::Instant::now();
    loop {
        if let Ok(Some(_status)) = child.try_wait() {
            break;
        }
        if start.elapsed() > Duration::from_secs(5) {
            let _ = child.kill();
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    reader_handle.join().unwrap()
}

/// A syntactically INVALID JSON line (parse error) must, per JSON-RPC 2.0,
/// produce a response `{"jsonrpc":"2.0","id":null,"error":{"code":-32700,...}}`.
#[test]
fn malformed_json_line_gets_parse_error_response() {
    // One broken line, followed by one well-formed request so we can prove
    // the process is still alive and responsive afterwards.
    let input = "{this is not valid json\n\
                  {\"jsonrpc\":\"2.0\",\"id\":42,\"method\":\"tools/list\",\"params\":{}}\n";

    let lines = run_mcp(input);

    // FIXED: the malformed line now gets its own -32700 parse-error
    // response, followed by the normal response to the valid request.
    assert_eq!(
        lines.len(),
        2,
        "expected two responses: a parse-error for the malformed line and \
         a normal response for the valid request; got {:?}",
        lines
    );

    let parse_err: serde_json::Value = serde_json::from_str(&lines[0]).unwrap();
    assert_eq!(parse_err["id"], serde_json::Value::Null);
    assert_eq!(parse_err["error"]["code"], -32700);

    let resp: serde_json::Value = serde_json::from_str(&lines[1]).unwrap();
    assert_eq!(resp["id"], 42);
}

/// A JSON-RPC 2.0 *batch* request (a top-level JSON array of request
/// objects) is syntactically valid and each element carries a real `id`
/// that a client will block on. The server previously keyed
/// response-vs-notification dispatch off `msg.get("id")`, which returns
/// `None` for a top-level array (arrays have no `"id"` key), so the whole
/// batch was misclassified as a notification and silently discarded.
/// Batching is explicitly not supported, so the server now returns a
/// diagnostic error instead of silence.
#[test]
fn batch_request_with_real_id_gets_invalid_request_error() {
    let input = "[{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\",\"params\":{}}]\n";

    let lines = run_mcp(input);

    // FIXED: batch requests now get a real -32600 error response instead
    // of being silently dropped.
    assert_eq!(
        lines.len(),
        1,
        "expected exactly one error response for the batch request; got {:?}",
        lines
    );
    let resp: serde_json::Value = serde_json::from_str(&lines[0]).unwrap();
    assert_eq!(resp["id"], serde_json::Value::Null);
    assert_eq!(resp["error"]["code"], -32600);
}
