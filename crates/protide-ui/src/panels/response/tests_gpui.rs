#[cfg(test)]
mod tests {
    use gpui::TestAppContext;
    use crate::panels::response::{ResponseData, ResponsePanel};

    fn init_theme(cx: &mut TestAppContext) {
        cx.update(|cx| {
            cx.set_global(gpui_component::Theme::default());
            crate::theme::init(cx);
        });
    }

    #[gpui::test]
    async fn test_response_panel_initial_state(cx: &mut TestAppContext) {
        init_theme(cx);
        let (panel, _cx) = cx.add_window_view(|window, cx| ResponsePanel::new(window, cx));
        cx.run_until_parked();
        panel.read_with(cx, |p, _cx| {
            assert!(p.response.is_none());
            assert!(!p.loading);
            assert!(p.error.is_none());
            assert_eq!(p.active_tab, 0);
        });
    }

    #[gpui::test]
    async fn test_response_panel_set_loading(cx: &mut TestAppContext) {
        init_theme(cx);
        let (panel, _cx) = cx.add_window_view(|window, cx| ResponsePanel::new(window, cx));
        panel.update(cx, |p, cx| p.set_loading(cx));
        cx.run_until_parked();
        panel.read_with(cx, |p, _cx| {
            assert!(p.loading);
            assert!(p.response.is_none());
        });
    }

    #[gpui::test]
    async fn test_response_panel_set_error_clears_loading(cx: &mut TestAppContext) {
        init_theme(cx);
        let (panel, _cx) = cx.add_window_view(|window, cx| ResponsePanel::new(window, cx));
        panel.update(cx, |p, cx| p.set_loading(cx));
        panel.update(cx, |p, cx| p.set_error("connection refused".to_string(), cx));
        cx.run_until_parked();
        panel.read_with(cx, |p, _cx| {
            assert!(!p.loading);
            assert_eq!(p.error.as_deref(), Some("connection refused"));
        });
    }

    #[gpui::test]
    async fn test_response_panel_set_response(cx: &mut TestAppContext) {
        init_theme(cx);
        let (panel, _cx) = cx.add_window_view(|window, cx| ResponsePanel::new(window, cx));
        let data = ResponseData {
            status: 200,
            status_text: "OK".to_string(),
            headers: vec![("content-type".to_string(), "application/json".to_string())],
            body: r#"{"ok":true}"#.to_string(),
            time: std::time::Duration::from_millis(42),
            size: 11,
        };
        panel.update(cx, |p, cx| p.set_response(data, cx));
        cx.run_until_parked();
        panel.read_with(cx, |p, _cx| {
            let resp = p.response.as_ref().unwrap();
            assert_eq!(resp.status, 200);
            assert!(!p.loading);
            assert!(p.error.is_none());
        });
    }

    #[gpui::test]
    async fn test_response_panel_set_response_clears_error(cx: &mut TestAppContext) {
        init_theme(cx);
        let (panel, _cx) = cx.add_window_view(|window, cx| ResponsePanel::new(window, cx));
        panel.update(cx, |p, cx| p.set_error("old error".to_string(), cx));
        let data = ResponseData {
            status: 201,
            status_text: "Created".to_string(),
            headers: vec![],
            body: "{}".to_string(),
            time: std::time::Duration::ZERO,
            size: 2,
        };
        panel.update(cx, |p, cx| p.set_response(data, cx));
        cx.run_until_parked();
        panel.read_with(cx, |p, _cx| {
            assert!(p.error.is_none());
            assert!(p.response.is_some());
        });
    }

    // ── G4: ConsolePanel eviction at MAX_ENTRIES ────────────────────────────

    #[gpui::test]
    async fn test_console_log_eviction_at_max(cx: &mut TestAppContext) {
        use gpui::AppContext as _;
        use crate::panels::console::{ConsolePanel, ConsoleEntry, MAX_ENTRIES};
        let panel = cx.new(ConsolePanel::new);
        panel.update(cx, |p, cx| {
            for i in 0..MAX_ENTRIES + 5 {
                p.log(ConsoleEntry::team(format!("msg-{}", i)), cx);
            }
        });
        cx.run_until_parked();
        panel.read_with(cx, |p, _| {
            assert_eq!(p.entry_count(), MAX_ENTRIES);
            // Newest entry should be msg-504 (MAX_ENTRIES + 4 = 504)
            let back = p.entries.back().unwrap();
            assert!(
                back.url.ends_with(&format!("{}", MAX_ENTRIES + 4)),
                "expected back url to end with {}, got: {}",
                MAX_ENTRIES + 4,
                back.url
            );
            // The first 5 entries (msg-0 .. msg-4) should have been evicted;
            // msg-5 is now the oldest entry at the front.
            let front = p.entries.front().unwrap();
            assert!(
                front.url.ends_with("5"),
                "expected msg-5 at front, got: {}",
                front.url
            );
        });
    }

    // ── G5: ResponsePanel JSON collapse reduces row count ──────────────────

    #[gpui::test]
    async fn test_json_tree_collapse_reduces_rows(cx: &mut TestAppContext) {
        init_theme(cx);
        let (panel, _cx) = cx.add_window_view(|window, cx| ResponsePanel::new(window, cx));
        panel.update(cx, |p, cx| {
            p.set_response(ResponseData {
                status: 200,
                status_text: "OK".to_string(),
                headers: vec![("content-type".to_string(), "application/json".to_string())],
                body: r#"{"a":{"b":1,"c":2},"d":3}"#.to_string(),
                time: std::time::Duration::ZERO,
                size: 25,
            }, cx);
        });
        cx.run_until_parked();
        let rows_before = panel.read_with(cx, |p, _| p.json_rows.len());
        assert!(rows_before > 0, "expected JSON rows after set_response");
        panel.update(cx, |p, cx| p.toggle_json_collapse("/a".to_string(), cx));
        cx.run_until_parked();
        let rows_after = panel.read_with(cx, |p, _| p.json_rows.len());
        assert!(
            rows_after < rows_before,
            "collapse must reduce row count: before={}, after={}",
            rows_before,
            rows_after
        );
    }
}
