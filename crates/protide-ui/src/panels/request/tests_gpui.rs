//! Interaction tests that drive real render()-registered on_click/dropdown
//! wiring via GPUI's in-process input simulation (`simulate_click`), not just
//! direct state mutation. This is the pattern GPUI's own maintainers (Zed)
//! use for their own component tests - see `debug_selector`/`debug_bounds`
//! in the vendored `gpui` crate. It requires no OS input injection and no
//! display server, unlike the xdotool/ydotool approach that doesn't work in
//! this sandbox.

#[cfg(test)]
mod tests {
    use crate::panels::request::RequestPanel;
    use crate::panels::request_types::{
        BodyType, FormField, FormFieldType, GrpcMethodInfo, GrpcStreamingType, RequestMode,
    };
    use crate::panels::response::ResponsePanel;
    use crate::test_support::init_theme;
    use gpui::{AppContext, Modifiers, TestAppContext};
    use protide_core::codegen::Language as CodegenLanguage;
    use protide_core::execution::ws::TungsteniteExecutor;

    type TestPanel = RequestPanel<TungsteniteExecutor>;

    #[gpui::test]
    async fn test_mode_dropdown_click_opens_and_selects(cx: &mut TestAppContext) {
        init_theme(cx);
        let (panel, cx) = cx.add_window_view(|window, cx| {
            let response_panel = cx.new(|cx| ResponsePanel::new(window, cx));
            TestPanel::new(window, cx, response_panel)
        });
        cx.run_until_parked();

        panel.read_with(cx, |p, _| assert_eq!(p.request_mode, RequestMode::Http));
        panel.read_with(cx, |p, _| assert!(!p.mode_dropdown_open));

        let selector_bounds = cx
            .debug_bounds("mode-selector")
            .expect("mode-selector should be rendered in the URL bar");
        cx.simulate_click(selector_bounds.center(), Modifiers::none());
        cx.run_until_parked();

        panel.read_with(cx, |p, _| {
            assert!(
                p.mode_dropdown_open,
                "clicking the mode selector should open the dropdown"
            );
        });

        let ws_row_bounds = cx
            .debug_bounds("mode-WebSocket")
            .expect("WebSocket row should be rendered while the dropdown is open");
        cx.simulate_click(ws_row_bounds.center(), Modifiers::none());
        cx.run_until_parked();

        panel.read_with(cx, |p, _| {
            assert_eq!(
                p.request_mode,
                RequestMode::WebSocket,
                "clicking the WebSocket row should switch modes"
            );
            assert!(
                !p.mode_dropdown_open,
                "selecting a mode should close the dropdown"
            );
        });
    }

    #[gpui::test]
    async fn test_grpc_service_and_method_picker_click_flow(cx: &mut TestAppContext) {
        init_theme(cx);
        let (panel, cx) = cx.add_window_view(|window, cx| {
            let response_panel = cx.new(|cx| ResponsePanel::new(window, cx));
            TestPanel::new(window, cx, response_panel)
        });
        cx.run_until_parked();

        // Arrange: seed parsed proto data directly (bypassing real file I/O,
        // which isn't the concern of this test) and switch into gRPC mode so
        // the picker actually renders.
        panel.update(cx, |p, cx| {
            p.grpc_services = vec![
                "greeter.Greeter".to_string(),
                "employee.EmployeeService".to_string(),
            ];
            p.grpc_methods = vec![
                GrpcMethodInfo {
                    full_name: "greeter.Greeter/SayHello".to_string(),
                    streaming_type: GrpcStreamingType::Unary,
                },
                GrpcMethodInfo {
                    full_name: "greeter.Greeter/SayHellos".to_string(),
                    streaming_type: GrpcStreamingType::ServerStreaming,
                },
                GrpcMethodInfo {
                    full_name: "employee.EmployeeService/GetEmployee".to_string(),
                    streaming_type: GrpcStreamingType::Unary,
                },
            ];
            p.grpc_service = Some("greeter.Greeter".to_string());
            p.grpc_method = Some(GrpcMethodInfo {
                full_name: "greeter.Greeter/SayHello".to_string(),
                streaming_type: GrpcStreamingType::Unary,
            });
            p.set_request_mode(RequestMode::Grpc, cx);
        });
        cx.run_until_parked();

        // Act: open the service picker and switch to the second service -
        // this exercises the exact on_click wiring that was previously
        // missing entirely (the picker existed in state but nothing in
        // render() ever called toggle_grpc_service_picker/select_grpc_service).
        let svc_bounds = cx
            .debug_bounds("grpc-service-selector")
            .expect("service selector should render in gRPC mode");
        cx.simulate_click(svc_bounds.center(), Modifiers::none());
        cx.run_until_parked();
        panel.read_with(cx, |p, _| assert!(p.grpc_service_picker_open));

        let row_bounds = cx
            .debug_bounds("grpc-service-employee.EmployeeService")
            .expect("employee.EmployeeService row should render in the open picker");
        cx.simulate_click(row_bounds.center(), Modifiers::none());
        cx.run_until_parked();

        panel.read_with(cx, |p, _| {
            assert_eq!(p.grpc_service.as_deref(), Some("employee.EmployeeService"));
            assert!(
                !p.grpc_service_picker_open,
                "selecting a service should close its picker"
            );
            // select_grpc_service auto-picks the first method under the new service's prefix.
            assert_eq!(
                p.grpc_method.as_ref().map(|m| m.full_name.as_str()),
                Some("employee.EmployeeService/GetEmployee"),
            );
        });

        // Switch back to greeter.Greeter (directly - only the picker click
        // wiring is under test here, not this particular transition) and
        // exercise the method picker.
        panel.update(cx, |p, cx| {
            p.select_grpc_service("greeter.Greeter".to_string(), cx)
        });
        cx.run_until_parked();

        let method_bounds = cx
            .debug_bounds("grpc-method-selector")
            .expect("method selector should render");
        cx.simulate_click(method_bounds.center(), Modifiers::none());
        cx.run_until_parked();
        panel.read_with(cx, |p, _| assert!(p.grpc_method_picker_open));

        let hellos_bounds = cx
            .debug_bounds("grpc-method-greeter.Greeter/SayHellos")
            .expect("SayHellos row should render for the greeter.Greeter service");
        cx.simulate_click(hellos_bounds.center(), Modifiers::none());
        cx.run_until_parked();

        panel.read_with(cx, |p, _| {
            assert_eq!(
                p.grpc_method.as_ref().map(|m| m.full_name.as_str()),
                Some("greeter.Greeter/SayHellos")
            );
            assert_eq!(
                p.grpc_method.as_ref().map(|m| m.streaming_type),
                Some(GrpcStreamingType::ServerStreaming)
            );
            assert!(
                !p.grpc_method_picker_open,
                "selecting a method should close its picker"
            );
        });
    }

    #[gpui::test]
    async fn test_generate_code_includes_form_body_instead_of_dropping_it(cx: &mut TestAppContext) {
        init_theme(cx);
        let (panel, cx) = cx.add_window_view(|window, cx| {
            let response_panel = cx.new(|cx| ResponsePanel::new(window, cx));
            TestPanel::new(window, cx, response_panel)
        });
        cx.run_until_parked();

        panel.update(cx, |p, _cx| {
            p.url = "https://api.example.com/login".to_string();
            p.body_type = BodyType::Form;
            p.form_data = vec![
                FormField {
                    key: "username".to_string(),
                    value: "bob".to_string(),
                    field_type: FormFieldType::Text,
                    file_path: None,
                    enabled: true,
                },
                FormField {
                    key: "password".to_string(),
                    value: "secret".to_string(),
                    field_type: FormFieldType::Text,
                    file_path: None,
                    enabled: true,
                },
            ];
        });

        panel.update_in(cx, |p, window, cx| {
            p.generate_code(CodegenLanguage::Curl, window, cx);
        });
        cx.run_until_parked();

        panel.read_with(cx, |p, cx| {
            let generated = p.codegen_editor.read(cx).value().to_string();
            assert!(
                generated.contains("username=bob") && generated.contains("password=secret"),
                "generated code should contain the url-encoded form fields, got: {generated}",
            );
        });
    }

    #[gpui::test]
    async fn test_generate_code_flags_file_upload_instead_of_silently_dropping_it(
        cx: &mut TestAppContext,
    ) {
        init_theme(cx);
        let (panel, cx) = cx.add_window_view(|window, cx| {
            let response_panel = cx.new(|cx| ResponsePanel::new(window, cx));
            TestPanel::new(window, cx, response_panel)
        });
        cx.run_until_parked();

        panel.update(cx, |p, _cx| {
            p.url = "https://api.example.com/upload".to_string();
            p.body_type = BodyType::Form;
            p.form_data = vec![FormField {
                key: "file".to_string(),
                value: "report.pdf".to_string(),
                field_type: FormFieldType::File,
                file_path: Some("/tmp/report.pdf".into()),
                enabled: true,
            }];
        });

        panel.update_in(cx, |p, window, cx| {
            p.generate_code(CodegenLanguage::Python, window, cx);
        });
        cx.run_until_parked();

        panel.read_with(cx, |p, cx| {
            let generated = p.codegen_editor.read(cx).value().to_string();
            assert!(
                !generated.is_empty(),
                "a file-upload body should not silently produce an empty generated body",
            );
        });
    }
}

#[cfg(test)]
mod mode_and_tab_tests {
    use crate::panels::request::RequestPanel;
    use crate::panels::request_types::RequestMode;
    use crate::panels::response::ResponsePanel;
    use crate::session::RequestDraft;
    use crate::test_support::init_theme;
    use gpui::{AppContext as _, Entity, TestAppContext, VisualTestContext};
    use protide_core::execution::ws::TungsteniteExecutor;

    type TestPanel = RequestPanel<TungsteniteExecutor>;

    const ALL_MODES: [RequestMode; 6] = [
        RequestMode::Http,
        RequestMode::GraphQL,
        RequestMode::WebSocket,
        RequestMode::SocketIo,
        RequestMode::Grpc,
        RequestMode::Trpc,
    ];

    fn panel(cx: &mut TestAppContext) -> (Entity<TestPanel>, &mut VisualTestContext) {
        init_theme(cx);
        cx.add_window_view(|window, cx| {
            let response_panel = cx.new(|cx| ResponsePanel::new(window, cx));
            TestPanel::new(window, cx, response_panel)
        })
    }

    #[test]
    fn every_mode_declares_at_least_one_tab() {
        for mode in ALL_MODES {
            assert!(
                !mode.tab_labels().is_empty(),
                "{mode:?} has no tabs, so no tab content could ever render"
            );
        }
    }

    #[gpui::test]
    async fn switching_protocol_resets_the_active_tab_into_range(cx: &mut TestAppContext) {
        // HTTP has the most tabs; every other mode has fewer, so a stale index
        // would point past the end of the new tab bar and blank the editor.
        let (p, cx) = panel(cx);
        for mode in ALL_MODES {
            p.update(cx, |p, cx| {
                p.set_request_mode(RequestMode::Http, cx);
                p.active_tab = RequestMode::Http.tab_labels().len() - 1;
                p.set_request_mode(mode, cx);
            });
            p.read_with(cx, |p, _| {
                assert_eq!(p.request_mode, mode);
                assert!(
                    p.active_tab < mode.tab_labels().len(),
                    "{mode:?}: active_tab {} is outside its {} tabs",
                    p.active_tab,
                    mode.tab_labels().len()
                );
            });
        }
    }

    #[gpui::test]
    async fn re_selecting_the_current_protocol_keeps_the_open_tab(cx: &mut TestAppContext) {
        let (p, cx) = panel(cx);
        p.update(cx, |p, cx| {
            p.set_request_mode(RequestMode::Http, cx);
            p.active_tab = 3;
            p.set_request_mode(RequestMode::Http, cx);
            assert_eq!(p.active_tab, 3, "a no-op mode change must not lose the tab");
        });
    }

    #[gpui::test]
    async fn switching_protocol_drops_any_in_progress_field_edit(cx: &mut TestAppContext) {
        // `active_edit` names a field of the *old* tab set; leaving it set would
        // route keystrokes into a field the new mode no longer shows.
        let (p, cx) = panel(cx);
        p.update(cx, |p, cx| {
            p.set_request_mode(RequestMode::Http, cx);
            p.edit_selection = 2..5;
            p.set_request_mode(RequestMode::GraphQL, cx);
            assert!(p.active_edit.is_none());
            assert_eq!(p.edit_selection, 0..0);
        });
    }

    #[gpui::test]
    async fn protocol_switches_install_a_usable_default_url(cx: &mut TestAppContext) {
        let (p, cx) = panel(cx);
        p.update(cx, |p, cx| {
            p.set_request_mode(RequestMode::WebSocket, cx);
            assert!(
                p.url.starts_with("ws://") || p.url.starts_with("wss://"),
                "WebSocket mode left a non-ws URL: {}",
                p.url
            );
            p.set_request_mode(RequestMode::Grpc, cx);
            assert!(p.url.contains("grpc"), "gRPC mode left {}", p.url);
        });
    }

    #[gpui::test]
    async fn the_url_cursor_stays_inside_a_replaced_url(cx: &mut TestAppContext) {
        // The default URLs are installed with the caret at the end; a caret left
        // beyond the new text would index past it on the next keystroke.
        let (p, cx) = panel(cx);
        p.update(cx, |p, cx| {
            for mode in ALL_MODES {
                p.set_request_mode(mode, cx);
                let n = p.url.chars().count();
                assert!(
                    p.url_selection.start <= n && p.url_selection.end <= n,
                    "{mode:?}: selection {:?} outside a {n}-char url",
                    p.url_selection
                );
            }
        });
    }

    // FIXED: `restore_from_draft` copied `draft.active_tab` verbatim. A session
    // file whose tab index does not exist in the restored protocol - hand
    // edited, truncated mid-write, or written by a build with a different tab
    // set - left `active_tab` past the end of the tab bar, and
    // `render_tab_content` fell through to a blank `div()`: an empty request
    // editor with no tab highlighted and no indication why.
    #[gpui::test]
    async fn restoring_a_draft_clamps_a_tab_index_the_protocol_does_not_have(
        cx: &mut TestAppContext,
    ) {
        let (p, cx) = panel(cx);
        for (protocol, mode) in [
            ("websocket", RequestMode::WebSocket),
            ("socketio", RequestMode::SocketIo),
            ("grpc", RequestMode::Grpc),
            ("trpc", RequestMode::Trpc),
            ("graphql", RequestMode::GraphQL),
            ("http", RequestMode::Http),
        ] {
            let draft = RequestDraft {
                protocol: protocol.to_string(),
                active_tab: 999,
                url: "https://x.test".to_string(),
                method: "GET".to_string(),
                ..Default::default()
            };
            p.update(cx, |p, cx| p.restore_from_draft(&draft, cx));
            p.read_with(cx, |p, _| {
                assert_eq!(p.request_mode, mode);
                assert!(
                    p.active_tab < mode.tab_labels().len(),
                    "{protocol}: restored active_tab {} is outside its {} tabs",
                    p.active_tab,
                    mode.tab_labels().len()
                );
            });
        }
    }

    #[gpui::test]
    async fn an_unknown_protocol_in_a_draft_falls_back_to_http(cx: &mut TestAppContext) {
        let (p, cx) = panel(cx);
        for protocol in ["", "quic", "HTTP", "graphQL"] {
            let draft = RequestDraft {
                protocol: protocol.to_string(),
                ..Default::default()
            };
            p.update(cx, |p, cx| p.restore_from_draft(&draft, cx));
            p.read_with(cx, |p, _| {
                assert_eq!(p.request_mode, RequestMode::Http, "protocol {protocol:?}");
            });
        }
    }

    #[gpui::test]
    async fn a_restored_draft_leaves_the_url_caret_inside_the_url(cx: &mut TestAppContext) {
        let (p, cx) = panel(cx);
        let draft = RequestDraft {
            protocol: "http".to_string(),
            // Multi-byte: a byte-based caret would land past the character count.
            url: "https://x.test/日本語?q=🎉".to_string(),
            method: "POST".to_string(),
            ..Default::default()
        };
        p.update(cx, |p, cx| p.restore_from_draft(&draft, cx));
        p.read_with(cx, |p, _| {
            let n = p.url.chars().count();
            assert_eq!(p.url_selection, n..n, "caret must sit at the end in chars");
        });
    }

    #[gpui::test]
    async fn a_draft_round_trips_through_capture_and_restore(cx: &mut TestAppContext) {
        let (p, cx) = panel(cx);
        p.update(cx, |p, cx| {
            p.set_request_mode(RequestMode::GraphQL, cx);
            p.url = "https://api.test/graphql".to_string();
            p.active_tab = 2;
            p.graphql_operation_name = "Me".to_string();
        });
        let draft = p.read_with(cx, |p, cx| p.capture_draft(cx));
        p.update(cx, |p, cx| {
            p.set_request_mode(RequestMode::Http, cx);
            p.url = "https://elsewhere.test".to_string();
            p.graphql_operation_name = String::new();
            p.restore_from_draft(&draft, cx);
        });
        p.read_with(cx, |p, _| {
            assert_eq!(p.request_mode, RequestMode::GraphQL);
            assert_eq!(p.url, "https://api.test/graphql");
            assert_eq!(p.active_tab, 2);
            assert_eq!(p.graphql_operation_name, "Me");
        });
    }
}

#[cfg(test)]
mod url_double_click_tests {
    use crate::panels::request::RequestPanel;
    use crate::panels::response::ResponsePanel;
    use crate::test_support::init_theme;
    use gpui::{
        AppContext as _, Modifiers, MouseButton, MouseDownEvent, TestAppContext, point, px,
    };
    use protide_core::execution::ws::TungsteniteExecutor;
    use std::ops::Range;

    type TestPanel = RequestPanel<TungsteniteExecutor>;

    /// Double-click the URL bar at `char_index` and return the resulting
    /// selection, driving the real `render()`-registered `on_mouse_down` rather
    /// than calling `find_word_start` / `find_word_end` directly.
    ///
    /// `cx.simulate_click` hardcodes `click_count: 1`, so the double-click goes
    /// in as a raw `MouseDownEvent` - that count is exactly what
    /// `handle_url_mouse_down` branches on.
    async fn double_click_at(
        cx: &mut TestAppContext,
        url: &str,
        char_index: usize,
    ) -> Range<usize> {
        init_theme(cx);
        let (panel, cx) = cx.add_window_view(|window, cx| {
            let response_panel = cx.new(|cx| ResponsePanel::new(window, cx));
            TestPanel::new(window, cx, response_panel)
        });
        panel.update(cx, |p, cx| {
            p.url = url.to_string();
            cx.notify();
        });
        cx.run_until_parked();

        let bounds = cx
            .debug_bounds("url-input")
            .expect("url-input should be rendered in the URL bar");
        // `url_input_left` is captured during render by the canvas child, and
        // `index_for_x` divides by a fixed 7.8px char width - so aim at the
        // middle of the target character's cell.
        let left = panel.read_with(cx, |p, _| p.url_input_left);
        let x = left + char_index as f32 * 7.8 + 3.9;

        cx.simulate_event(MouseDownEvent {
            position: point(px(x), bounds.center().y),
            modifiers: Modifiers::none(),
            button: MouseButton::Left,
            click_count: 2,
            first_mouse: false,
        });
        cx.run_until_parked();

        panel.read_with(cx, |p, _| p.url_selection.clone())
    }

    const URL: &str = "https://api.example.com";

    #[gpui::test]
    async fn a_double_click_inside_a_word_selects_that_word(cx: &mut TestAppContext) {
        assert_eq!(
            double_click_at(cx, URL, 0).await,
            0..5,
            "expected \"https\""
        );
    }

    #[gpui::test]
    async fn a_double_click_on_a_separator_selects_the_separator_run(cx: &mut TestAppContext) {
        // The behaviour the word_select fix chose, asserted end-to-end through
        // real mouse wiring: "://" rather than the old "https://api".
        assert_eq!(double_click_at(cx, URL, 5).await, 5..8, "expected \"://\"");
    }
}
