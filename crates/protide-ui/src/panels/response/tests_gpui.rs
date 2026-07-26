#[cfg(test)]
mod tests {
    use crate::components::selectable_text::SelectionRange;
    use crate::panels::response::{ResponseData, ResponsePanel};
    use crate::test_support::init_theme;
    use gpui::TestAppContext;

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
        panel.update(cx, |p, cx| {
            p.set_error("connection refused".to_string(), cx)
        });
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
        use crate::panels::console::{ConsoleEntry, ConsolePanel, MAX_ENTRIES};
        use gpui::AppContext as _;
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
            p.set_response(
                ResponseData {
                    status: 200,
                    status_text: "OK".to_string(),
                    headers: vec![("content-type".to_string(), "application/json".to_string())],
                    body: r#"{"a":{"b":1,"c":2},"d":3}"#.to_string(),
                    time: std::time::Duration::ZERO,
                    size: 25,
                },
                cx,
            );
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

    // ── REGRESSION: copy_json_selection must not panic on multi-byte UTF-8 ─────
    // json_val_char_at_x (json.rs) used to return a char-count-based index but
    // store it directly into SelectionRange, whose fields are documented (and
    // used elsewhere, e.g. copy_json_selection / hdr_val_byte_at) as *byte*
    // offsets. For strings containing multi-byte UTF-8 chars the char-count no
    // longer matched a byte offset, so slicing `&text[so..eo]` in
    // copy_json_selection could land mid-character and panic.
    //
    // json_val_char_at_x now converts the pixel-derived char index to a real
    // byte offset via `char_indices().nth(...)` (mirroring hdr_val_byte_at), so
    // any offset it produces is always a valid char boundary. This test drives
    // that exact function with a pixel-x chosen to land inside the multi-byte
    // '日' character, and confirms the resulting byte offset is a valid
    // boundary that yields the correct substring instead of panicking.
    #[gpui::test]
    async fn test_copy_json_selection_multibyte_utf8_no_panic(cx: &mut TestAppContext) {
        init_theme(cx);
        let (panel, _cx) = cx.add_window_view(|window, cx| ResponsePanel::new(window, cx));
        panel.update(cx, |p, cx| {
            p.set_response(
                ResponseData {
                    status: 200,
                    status_text: "OK".to_string(),
                    headers: vec![("content-type".to_string(), "application/json".to_string())],
                    // display text for row 1 becomes: "日本語" (quote + 3x 3-byte chars + quote)
                    body: r#"{"a":"日本語"}"#.to_string(),
                    time: std::time::Duration::ZERO,
                    size: 20,
                },
                cx,
            );
        });
        cx.run_until_parked();
        // Row 0 = Open "{", Row 1 = Leaf "a": "日本語", Row 2 = Close "}"

        // Ask json_val_char_at_x for the byte offset at a pixel-x deep enough
        // to be "2 characters in" (quote=char0, 日=char1, so char_idx=2 would
        // land right after '日', mid-string if it were naively treated as a
        // byte offset -- '日' is 3 bytes, so byte offset 2 is invalid).
        let end_offset = panel.update(cx, |p, _cx| {
            use crate::panels::response::{CHEVRON_W, GUTTER_W, INDENT_W, JSON_CHAR_W};
            let bounds = p.json_tree_bounds.unwrap_or_default();
            let row = &p.json_rows[1];
            let key_chars = 1 + 4; // "a":
            let val_x = f32::from(bounds.origin.x)
                + GUTTER_W
                + (row.depth as f32) * INDENT_W
                + CHEVRON_W
                + (key_chars as f32) * JSON_CHAR_W;
            // 2.5 chars in: guarantees we've crossed past the '日' character
            // (whose char boundary starts at char_idx 1), landing on char_idx 2.
            let ex = gpui::px(val_x + 2.5 * JSON_CHAR_W);
            p.json_val_char_at_x(ex, 1)
        });

        // The returned offset must be a genuine byte offset: 0 (quote) + 1 (quote
        // byte) + 3 (bytes of '日') = 4, landing exactly on the boundary after '日'.
        let display = panel.read_with(cx, |p, _cx| p.json_row_display_text(1).to_string());
        assert!(
            display.is_char_boundary(end_offset),
            "json_val_char_at_x returned byte offset {} which is not a char boundary in {:?}",
            end_offset,
            display
        );
        assert_eq!(
            end_offset, 4,
            "expected byte offset just past the quote + '日' (1 + 3 bytes)"
        );

        panel.update(cx, |p, _cx| {
            p.json_sel = Some(SelectionRange::new(1, 0, 1, end_offset));
        });
        // Must not panic, and must produce the correctly-sliced substring on the clipboard.
        panel.update(cx, |p, cx| p.copy_json_selection(cx));
        panel.read_with(cx, |_p, cx| {
            let clip = cx.read_from_clipboard().and_then(|c| c.text());
            assert_eq!(
                clip.as_deref(),
                Some("\"日"),
                "expected the selection to cover the opening quote + '日'"
            );
        });
    }
}
