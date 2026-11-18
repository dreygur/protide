use std::sync::mpsc;

use super::sio::SioUiEvent;
use super::sio_codec::{encode_sio_connect, parse_eio_type, parse_sio_header};

/// Wait for the SIO CONNECT ack. Returns true on success, false on error.
/// Some servers send an EIO PING before the SIO CONNECT ack - we handle one
/// such ping transparently before giving up.
pub(super) async fn await_sio_connect_ack<W, R>(
    write: &mut W,
    read: &mut R,
    namespace: &str,
    event_tx: &mpsc::Sender<SioUiEvent>,
) -> bool
where
    W: futures_util::Sink<
            tokio_tungstenite::tungstenite::Message,
            Error = tokio_tungstenite::tungstenite::Error,
        > + Unpin,
    R: futures_util::Stream<
            Item = Result<
                tokio_tungstenite::tungstenite::Message,
                tokio_tungstenite::tungstenite::Error,
            >,
        > + Unpin,
{
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::Message;

    let mut pings_handled = 0u8;
    loop {
        match tokio::time::timeout(std::time::Duration::from_secs(10), read.next()).await {
            Ok(Some(Ok(Message::Text(text)))) => match parse_eio_type(&text) {
                Some(('4', sio)) => match parse_sio_header(sio) {
                    Some((0, _, _, _)) => return true,
                    Some((4, _, _, data)) => {
                        let msg = serde_json::from_str::<serde_json::Value>(data)
                            .ok()
                            .and_then(|v| {
                                v.get("message").and_then(|m| m.as_str()).map(String::from)
                            })
                            .unwrap_or_else(|| data.to_string());
                        let _ = event_tx.send(SioUiEvent::Error(format!("Connect error: {}", msg)));
                        return false;
                    }
                    _ => {
                        let _ = event_tx.send(SioUiEvent::Error(
                            "Unexpected SIO packet during connect".into(),
                        ));
                        return false;
                    }
                },
                // Some servers send EIO PINGs before the SIO CONNECT ack - reply and keep waiting
                Some(('2', probe)) => {
                    pings_handled += 1;
                    if pings_handled > 5 {
                        let _ = event_tx.send(SioUiEvent::Error(
                            "Too many EIO pings during connect".into(),
                        ));
                        return false;
                    }
                    let pong = format!("3{}", probe);
                    if write.send(Message::Text(pong.into())).await.is_err() {
                        let _ =
                            event_tx.send(SioUiEvent::Error("Write error during connect".into()));
                        return false;
                    }
                    // Re-send SIO CONNECT in case the server expects it after the ping
                    let connect_pkt = format!("4{}", encode_sio_connect(namespace));
                    let _ = write.send(Message::Text(connect_pkt.into())).await;
                }
                _ => {
                    let _ = event_tx.send(SioUiEvent::Error(
                        "Unexpected EIO packet during connect".into(),
                    ));
                    return false;
                }
            },
            _ => {
                let _ = event_tx.send(SioUiEvent::Error("SIO CONNECT ack timeout".into()));
                return false;
            }
        }
    }
}
