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
