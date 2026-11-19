//! A script loaded from a file runs only once the user trusts it; a script
//! the user wrote in the editor is trusted on send.

use crate::panels::ScriptTrust;
use crate::panels::explorer::ExplorerPanel;
use crate::panels::request::RequestPanel;
use crate::panels::response::ResponsePanel;
use crate::test_support::init_theme;
use gpui::{AppContext, Entity, TestAppContext, VisualTestContext, WeakEntity};
use protide_core::execution::ws::TungsteniteExecutor;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::{Duration, Instant};

type TestPanel = RequestPanel<TungsteniteExecutor>;

const SCRIPT: &str = "env.set('ran', 'yes');";

/// Answer one request on a loopback port with an empty 200, giving up
/// after a few seconds so a test that never sends leaves no thread behind.
fn one_shot_server() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("http://{}/", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            let Ok((mut stream, _)) = listener.accept() else {
                std::thread::sleep(Duration::from_millis(10));
                continue;
            };
            let _ = stream.set_nonblocking(false);
            let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
            let _ = stream.read(&mut [0u8; 4096]);
            let _ = stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
            return;
        }
    });
    url
}

/// A panel holding a request loaded from a file whose pre script is
/// `script`, with an explorer whose environment shows what the script set.
fn loaded_panel<'a>(
    cx: &'a mut TestAppContext,
    script: &str,
) -> (
    Entity<TestPanel>,
    Entity<ExplorerPanel>,
    &'a mut VisualTestContext,
) {
    init_theme(cx);
    cx.update(|cx| cx.set_global(crate::panels::RequestHistory::new()));
    let (panel, cx) = cx.add_window_view(|window, cx| {
        let response_panel = cx.new(|cx| ResponsePanel::new(window, cx));
        TestPanel::new(window, cx, response_panel)
    });
    let explorer = cx.new(|cx| ExplorerPanel::new(cx, WeakEntity::new_invalid()));
    let text = format!("GET {}\n\n# @pre-script\n{script}\n", one_shot_server());
    let req = http_parser::parse(&text).unwrap().remove(0);
    panel.update(cx, |p, cx| {
        p.set_explorer_panel(explorer.clone(), cx);
        p.load_from_parsed_request(&req, cx);
    });
    cx.run_until_parked();
    (panel, explorer, cx)
}

fn env_var(explorer: &Entity<ExplorerPanel>, cx: &mut VisualTestContext) -> Option<String> {
    explorer.read_with(cx, |e, _| {
        e.env_state()
            .active()
            .and_then(|env| env.get("ran").cloned())
    })
}

fn send(panel: &Entity<TestPanel>, cx: &mut VisualTestContext) {
    panel.update(cx, |p, cx| p.send_request(cx));
    cx.run_until_parked();
}

fn is_trusted(script: &str, cx: &mut VisualTestContext) -> bool {
    cx.update(|_, cx| cx.default_global::<ScriptTrust>().0.is_trusted(script))
}

#[gpui::test]
async fn a_loaded_script_runs_only_after_the_user_trusts_it(cx: &mut TestAppContext) {
    let (panel, explorer, cx) = loaded_panel(cx, SCRIPT);

    send(&panel, cx);
    panel.read_with(cx, |p, _| assert!(p.pending_trust.is_some() && !p.loading));
    panel.update(cx, |p, cx| p.resolve_trust(false, cx));
    cx.run_until_parked();
    assert_eq!(env_var(&explorer, cx), None, "a cancelled script ran");
    assert!(!is_trusted(SCRIPT, cx));

    send(&panel, cx);
    panel.update(cx, |p, cx| p.resolve_trust(true, cx));
    cx.run_until_parked();
    assert_eq!(env_var(&explorer, cx).as_deref(), Some("yes"));
    assert!(is_trusted(SCRIPT, cx));
}

#[gpui::test]
async fn a_script_the_user_edited_is_trusted_on_send(cx: &mut TestAppContext) {
    let (panel, explorer, cx) = loaded_panel(cx, "env.set('ran', 'no');");
    panel.update_in(cx, |p, window, cx| {
        p.pre_script_editor
            .update(cx, |s, cx| s.set_value(SCRIPT, window, cx));
    });

    send(&panel, cx);
    panel.read_with(cx, |p, _| assert!(p.pending_trust.is_none()));
    assert!(is_trusted(SCRIPT, cx));
    assert_eq!(env_var(&explorer, cx).as_deref(), Some("yes"));
}
