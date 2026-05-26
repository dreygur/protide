use super::*;
use gpui_component::input::InputState;

impl<E: WebSocketExecutor> RequestPanel<E> {
    pub fn new(window: &mut Window, cx: &mut Context<Self>, response_panel: Entity<ResponsePanel>) -> Self {
        let url = "https://httpbin.org/post".to_string();
        let url_len = url.len();
        let initial_body = "{\n  \"name\": \"Protide\",\n  \"version\": \"0.1.0\"\n}";
        let body_editor = cx.new(|cx| {
            InputState::new(window, cx).multi_line(true).code_editor("json").line_number(true).default_value(initial_body)
        });
        let pre_script_editor = cx.new(|cx| {
            InputState::new(window, cx).multi_line(true).code_editor("javascript").line_number(true)
        });
        let post_script_editor = cx.new(|cx| {
            InputState::new(window, cx).multi_line(true).code_editor("javascript").line_number(true)
        });
        let tests_editor = cx.new(|cx| {
            InputState::new(window, cx).multi_line(true).code_editor("javascript").line_number(true)
        });
        let graphql_query_editor = cx.new(|cx| {
            InputState::new(window, cx).multi_line(true).code_editor("graphql").line_number(true).default_value("query {\n  \n}")
        });
        let graphql_variables_editor = cx.new(|cx| {
            InputState::new(window, cx).multi_line(true).code_editor("json").line_number(true).default_value("{}")
        });
        let ws_message_editor = cx.new(|cx| {
            InputState::new(window, cx).multi_line(true).code_editor("json").line_number(true).default_value("{\"type\": \"hello\"}")
        });
        let grpc_message_editor = cx.new(|cx| {
            InputState::new(window, cx).multi_line(true).code_editor("json").line_number(true).default_value("{}")
        });
        let trpc_params_editor = cx.new(|cx| {
            InputState::new(window, cx).multi_line(true).code_editor("json").line_number(true).default_value("{}")
        });
        let trpc_pg_search_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("Search procedures...")
        });
        let trpc_pg_result_viewer = cx.new(|cx| {
            InputState::new(window, cx).multi_line(true).code_editor("json").line_number(true)
        });
        let trpc_pg_add_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("router.name")
        });
        let trpc_pg_import_url_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("https://api.example.com/trpc")
        });
        let trpc_pg_edit_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("router.name")
        });
        let trpc_pg_group_edit_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("router")
        });
        let sio_payload_editor = cx.new(|cx| {
            InputState::new(window, cx).multi_line(true).code_editor("json").default_value("{}")
        });
        let codegen_editor = cx.new(|cx| {
            InputState::new(window, cx).multi_line(true).line_number(true)
        });
        let import_editor = cx.new(|cx| {
            InputState::new(window, cx).multi_line(true)
        });
        Self {
            active_tab: 0,
            method: HttpMethod::Post,
            url,
            url_selection: url_len..url_len,
            method_dropdown_open: false,
            mode_dropdown_open: false,
            url_focus: cx.focus_handle(),
            is_selecting: false,
            url_input_left: 0.0,
            url_input_width: 400.0,
            url_scroll_offset: 0.0,
            _edit_blur_sub: None,
            response_panel,
            loading: false,
            headers: vec![
                KeyValuePair {
                    key: "Content-Type".to_string(),
                    value: "application/json".to_string(),
                    enabled: true,
                },
                KeyValuePair::default(),
                KeyValuePair::default(),
            ],
            params: vec![
                KeyValuePair::default(),
                KeyValuePair::default(),
                KeyValuePair::default(),
            ],
            form_data: vec![FormField::default()],
            body: initial_body.to_string(),
            body_type: BodyType::Json,
            binary_file_path: None,
            syncing_params: false,
            auth_type: AuthType::None,
            bearer_token: String::new(),
            basic_username: String::new(),
            basic_password: String::new(),
            api_key_name: String::new(),
            api_key_value: String::new(),
            api_key_location: ApiKeyLocation::Header,
            active_edit: None,
            edit_selection: 0..0,
            edit_is_selecting: false,
            edit_input_origins: std::collections::HashMap::new(),
            url_undo_stack: Vec::new(),
            url_redo_stack: Vec::new(),
            edit_undo_stack: Vec::new(),
            edit_redo_stack: Vec::new(),
            skip_blur: false,
            edit_focus: cx.focus_handle(),
            body_focus: cx.focus_handle(),
            explorer_panel: None,
            body_editor,
            pre_script: String::new(),
            post_script: String::new(),
            tests: String::new(),
            pre_script_editor,
            post_script_editor,
            tests_editor,
            variable_extractions: Vec::new(),
            codegen_content: None,
            codegen_language: CodegenLanguage::Curl,
            codegen_editor,
            import_modal_open: false,
            import_text: String::new(),
            import_error: None,
            import_editor,
            request_mode: RequestMode::Http,
            graphql_query_editor,
            graphql_variables_editor,
            graphql_operation_name: String::new(),
            ws_state: WsConnectionState::Disconnected,
            ws_messages: WsRingBuffer::default(),
            ws_message_editor,
            ws_send_tx: None,
            ws_compose_h: 120.0,
            ws_compose_drag: None,
            ws_scroll: gpui::ScrollHandle::new(),
            grpc_message_editor,
            grpc_metadata: vec![KeyValuePair::default()],
            grpc_proto_path: None,
            grpc_proto_content: String::new(),
            grpc_services: Vec::new(),
            grpc_service: None,
            grpc_methods: Vec::new(),
            grpc_method: None,
            trpc_procedure: String::new(),
            trpc_params_editor,
            trpc_pg_procedures: vec![],
            trpc_pg_selected: None,
            trpc_pg_loading: false,
            trpc_pg_response: None,
            trpc_pg_error: None,
            trpc_pg_status: None,
            trpc_pg_elapsed: None,
            trpc_pg_search_input,
            trpc_pg_result_viewer,
            trpc_pg_add_input,
            trpc_pg_add_kind: TrpcProcKind::Query,
            trpc_pg_sidebar_w: 220.0,
            trpc_pg_sidebar_drag: None,
            trpc_pg_schema_loading: false,
            trpc_pg_schema_error: None,
            trpc_pg_import_url_input,
            trpc_pg_show_import_url: false,
            trpc_pg_editing: None,
            trpc_pg_edit_input,
            trpc_pg_edit_kind: TrpcProcKind::Query,
            trpc_pg_editing_group: None,
            trpc_pg_group_edit_input,
            sio_state: SioConnectionState::Disconnected,
            sio_messages: SioRingBuffer::default(),
            sio_namespace: "/".to_string(),
            sio_event_name: "message".to_string(),
            sio_want_ack: false,
            sio_next_ack_id: 1,
            sio_payload_editor,
            sio_send_tx: None,
            sio_room_name: String::new(),
            sio_active_rooms: Vec::new(),
            kv_col_key_w: 150.0,
            kv_col_drag: None,
            script_pre_open: true,
            script_post_open: true,
            script_tests_open: true,
            script_pre_h: crate::prefs::get_f32("request.script_pre_h", 160.0),
            script_post_h: crate::prefs::get_f32("request.script_post_h", 160.0),
            drag_script_pre: None,
            drag_script_post: None,
            current_file: None,
            save_feedback: false,
            custom_method_input: String::new(),
            custom_method_focus: cx.focus_handle(),
            graphql_schema: GraphqlSchemaState::Idle,
            graphql_schema_search: String::new(),
            console_panel: None,
            csv_path: None,
            data_results: Vec::new(),
            data_running: false,
            timeout_input: cx.new(|cx| {
                InputState::new(window, cx).default_value("30").placeholder("30")
            }),
            verify_ssl: true,
            impersonate_browser: false,
            kv_row_drag: None,
            kv_row_drag_over: None,
            form_row_drag: None,
            form_row_drag_over: None,
            editor_pending: Vec::new(),
            _executor: PhantomData,
        }
    }
}
