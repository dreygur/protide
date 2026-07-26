#[cfg(test)]
mod tests {
    use crate::components::selectable_text::SelectionRange;
    use crate::panels::response::{ResponseData, ResponsePanel};
    use crate::test_support::init_theme;
    use gpui::TestAppContext;

    #[gpui::test]
    async fn test_response_panel_initial_state(cx: &mut TestAppContext) {
        init_theme(cx);
        let (panel, _cx) = cx.add_window_view(ResponsePanel::new);
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
        let (panel, _cx) = cx.add_window_view(ResponsePanel::new);
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
        let (panel, _cx) = cx.add_window_view(ResponsePanel::new);
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
        let (panel, _cx) = cx.add_window_view(ResponsePanel::new);
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
        let (panel, _cx) = cx.add_window_view(ResponsePanel::new);
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
        let (panel, _cx) = cx.add_window_view(ResponsePanel::new);
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
        let (panel, _cx) = cx.add_window_view(ResponsePanel::new);
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

#[cfg(test)]
mod header_selection_tests {
    use crate::panels::response::{HdrSel, ResponseData, ResponsePanel};
    use crate::test_support::init_theme;
    use gpui::TestAppContext;

    fn response(headers: Vec<(&str, &str)>) -> ResponseData {
        ResponseData {
            status: 200,
            status_text: "OK".to_string(),
            headers: headers
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            body: String::new(),
            time: std::time::Duration::ZERO,
            size: 0,
        }
    }

    /// Pixel-x that lands `chars` characters into the header value column.
    fn value_x(panel_col1_w: f32, chars: f32) -> gpui::Pixels {
        use crate::panels::response::{HDR_CHAR_W, HDR_PADDING, HDR_SPACER_W};
        gpui::px(panel_col1_w + HDR_SPACER_W + HDR_PADDING + chars * HDR_CHAR_W)
    }

    // FIXED: `set_response` reset `json_sel` but left `hdr_sel` alone, so a
    // header selection captured against one response survived into the next.
    // Its byte offsets then indexed a *different* string: at best the wrong span
    // was highlighted and copied, at worst `selectable_text_element` sliced
    // mid-character and panicked the window.
    #[gpui::test]
    async fn a_new_response_clears_the_previous_header_selection(cx: &mut TestAppContext) {
        init_theme(cx);
        let (panel, _cx) = cx.add_window_view(ResponsePanel::new);
        panel.update(cx, |p, cx| {
            p.set_response(response(vec![("x-note", "日本語です")]), cx);
            p.hdr_sel = Some(HdrSel {
                row: 0,
                range: (0, 6),
                selecting: false,
            });
        });
        cx.run_until_parked();
        panel.update(cx, |p, cx| {
            p.set_response(response(vec![("x-note", "abcdeé")]), cx)
        });
        cx.run_until_parked();
        panel.read_with(cx, |p, _| {
            assert!(
                p.hdr_sel.is_none(),
                "a header selection must not outlive the response it was made in"
            );
        });
    }

    #[gpui::test]
    async fn header_offsets_land_on_char_boundaries_of_multibyte_values(cx: &mut TestAppContext) {
        init_theme(cx);
        let (panel, _cx) = cx.add_window_view(ResponsePanel::new);
        // 3-byte CJK, a combining mark and a 4-byte emoji in one value.
        let value = "日本語 e\u{0301}🎉 tail";
        panel.update(cx, |p, cx| {
            p.set_response(response(vec![("x-note", value)]), cx)
        });
        cx.run_until_parked();
        panel.read_with(cx, |p, _| {
            let col1 = p.resp_header_col1_w;
            for tenths in 0..200 {
                let byte = p.hdr_val_byte_at(value_x(col1, tenths as f32 * 0.5), 0);
                assert!(
                    value.is_char_boundary(byte),
                    "byte {byte} is not a char boundary in {value:?}"
                );
                let _ = &value[..byte]; // must not panic
            }
        });
    }

    #[gpui::test]
    async fn a_click_left_of_the_value_column_selects_the_first_character(cx: &mut TestAppContext) {
        init_theme(cx);
        let (panel, _cx) = cx.add_window_view(ResponsePanel::new);
        panel.update(cx, |p, cx| {
            p.set_response(response(vec![("x", "日本語")]), cx)
        });
        cx.run_until_parked();
        panel.read_with(cx, |p, _| {
            // Negative offsets are clamped, not wrapped into a huge usize.
            assert_eq!(p.hdr_val_byte_at(gpui::px(-9999.0), 0), 0);
            assert_eq!(p.hdr_val_byte_at(gpui::px(0.0), 0), 0);
        });
    }

    #[gpui::test]
    async fn a_click_past_the_end_of_a_value_selects_up_to_its_end(cx: &mut TestAppContext) {
        init_theme(cx);
        let (panel, _cx) = cx.add_window_view(ResponsePanel::new);
        let value = "日本語";
        panel.update(cx, |p, cx| p.set_response(response(vec![("x", value)]), cx));
        cx.run_until_parked();
        panel.read_with(cx, |p, _| {
            let col1 = p.resp_header_col1_w;
            assert_eq!(p.hdr_val_byte_at(value_x(col1, 500.0), 0), value.len());
        });
    }

    #[gpui::test]
    async fn an_offset_for_a_row_that_no_longer_exists_is_zero(cx: &mut TestAppContext) {
        init_theme(cx);
        let (panel, _cx) = cx.add_window_view(ResponsePanel::new);
        panel.update(cx, |p, cx| p.set_response(response(vec![]), cx));
        cx.run_until_parked();
        panel.read_with(cx, |p, _| {
            assert_eq!(p.hdr_val_byte_at(gpui::px(9999.0), 7), 0);
        });
    }
}

#[cfg(test)]
mod json_selection_tests {
    use crate::panels::response::{
        CHEVRON_W, GUTTER_W, INDENT_W, JSON_CHAR_W, ResponseData, ResponsePanel, SelectionRange,
    };
    use crate::test_support::init_theme;
    use gpui::TestAppContext;

    fn json_response(body: &str) -> ResponseData {
        ResponseData {
            status: 200,
            status_text: "OK".to_string(),
            headers: vec![("content-type".to_string(), "application/json".to_string())],
            body: body.to_string(),
            time: std::time::Duration::ZERO,
            size: body.len(),
        }
    }

    /// Pixel-x `chars` characters into row `row`'s value column, given the key's
    /// rendered width in *characters* (`"key": ` is `key_chars + 4`).
    fn value_x(panel: &ResponsePanel, row: usize, key_chars: usize, chars: f32) -> gpui::Pixels {
        let bounds = panel.json_tree_bounds.unwrap_or_default();
        let depth = panel.json_rows[row].depth as f32;
        gpui::px(
            f32::from(bounds.origin.x)
                + GUTTER_W
                + depth * INDENT_W
                + CHEVRON_W
                + ((key_chars + 4) as f32) * JSON_CHAR_W
                + chars * JSON_CHAR_W,
        )
    }

    // FIXED: `json_val_char_at_x` measured the key column as `k.len() + 4` -
    // `len()` is *bytes*, but the figure is multiplied by JSON_CHAR_W, a
    // per-*character* width. A key with any non-ASCII character (a CJK field
    // name, an emoji) therefore over-measured its own column, so every click on
    // that row's value resolved several characters too far left - selecting and
    // copying the wrong span. Same byte-vs-char confusion as the earlier
    // json_val_char_at_x regression, one line up.
    #[gpui::test]
    async fn clicks_resolve_correctly_on_rows_with_a_multibyte_key(cx: &mut TestAppContext) {
        init_theme(cx);
        let (panel, _cx) = cx.add_window_view(ResponsePanel::new);
        panel.update(cx, |p, cx| {
            p.set_response(json_response(r#"{"日本語":"abcdefgh"}"#), cx)
        });
        cx.run_until_parked();
        panel.read_with(cx, |p, _| {
            // Row 1 is the leaf; its display text is the quoted value.
            assert_eq!(p.json_row_display_text(1), "\"abcdefgh\"");
            // "日本語" renders as 3 characters, so the key column is 3 + 4 wide.
            for (chars_in, want) in [(0.5, 0), (1.5, 1), (2.5, 2), (5.5, 5)] {
                assert_eq!(
                    p.json_val_char_at_x(value_x(p, 1, 3, chars_in), 1),
                    want,
                    "click {chars_in} chars into the value of a CJK-keyed row"
                );
            }
        });
    }

    #[gpui::test]
    async fn an_ascii_key_still_resolves_the_same_way(cx: &mut TestAppContext) {
        init_theme(cx);
        let (panel, _cx) = cx.add_window_view(ResponsePanel::new);
        panel.update(cx, |p, cx| {
            p.set_response(json_response(r#"{"abc":"abcdefgh"}"#), cx)
        });
        cx.run_until_parked();
        panel.read_with(cx, |p, _| {
            for (chars_in, want) in [(0.5, 0), (2.5, 2), (5.5, 5)] {
                assert_eq!(p.json_val_char_at_x(value_x(p, 1, 3, chars_in), 1), want);
            }
        });
    }

    #[gpui::test]
    async fn json_offsets_are_always_char_boundaries(cx: &mut TestAppContext) {
        init_theme(cx);
        let (panel, _cx) = cx.add_window_view(ResponsePanel::new);
        panel.update(cx, |p, cx| {
            p.set_response(json_response(r#"{"k🎉":"日本 é 🎉 tail"}"#), cx)
        });
        cx.run_until_parked();
        panel.read_with(cx, |p, _| {
            let text = p.json_row_display_text(1).to_string();
            for tenths in 0..300 {
                let byte = p.json_val_char_at_x(value_x(p, 1, 2, tenths as f32 * 0.5), 1);
                assert!(
                    text.is_char_boundary(byte),
                    "byte {byte} is not a char boundary in {text:?}"
                );
            }
        });
    }

    #[gpui::test]
    async fn a_click_on_a_row_that_does_not_exist_yields_zero(cx: &mut TestAppContext) {
        init_theme(cx);
        let (panel, _cx) = cx.add_window_view(ResponsePanel::new);
        panel.update(cx, |p, cx| p.set_response(json_response("{}"), cx));
        cx.run_until_parked();
        panel.read_with(cx, |p, _| {
            assert_eq!(p.json_val_char_at_x(gpui::px(9999.0), 999), 0);
            assert_eq!(p.json_row_display_text(999), "");
        });
    }

    #[gpui::test]
    async fn copying_any_selection_of_a_multibyte_tree_never_panics(cx: &mut TestAppContext) {
        init_theme(cx);
        let (panel, _cx) = cx.add_window_view(ResponsePanel::new);
        panel.update(cx, |p, cx| {
            p.set_response(
                json_response(r#"{"a":"日本語","b":{"c":"é🎉"},"d":[1,"ü"]}"#),
                cx,
            )
        });
        cx.run_until_parked();
        let rows = panel.read_with(cx, |p, _| p.json_rows.len());
        assert!(rows > 4, "expected a multi-row tree, got {rows}");
        // Every row pair, with offsets deliberately overshooting each row's length.
        for sr in 0..rows + 1 {
            for er in 0..rows + 1 {
                for (so, eo) in [(0, 1), (1, 2), (2, 4), (3, 99), (99, 0), (0, usize::MAX)] {
                    panel.update(cx, |p, cx| {
                        p.json_sel = Some(SelectionRange::new(sr, so, er, eo));
                        p.copy_json_selection(cx);
                    });
                }
            }
        }
    }

    #[gpui::test]
    async fn a_new_response_clears_the_previous_json_selection(cx: &mut TestAppContext) {
        init_theme(cx);
        let (panel, _cx) = cx.add_window_view(ResponsePanel::new);
        panel.update(cx, |p, cx| {
            p.set_response(json_response(r#"{"a":"日本語"}"#), cx);
            p.json_sel = Some(SelectionRange::new(1, 0, 1, 6));
            p.json_selecting = true;
        });
        cx.run_until_parked();
        panel.update(cx, |p, cx| {
            p.set_response(json_response(r#"{"a":"abcdeé"}"#), cx)
        });
        cx.run_until_parked();
        panel.read_with(cx, |p, _| {
            assert!(p.json_sel.is_none());
            assert!(!p.json_selecting);
        });
    }
}
