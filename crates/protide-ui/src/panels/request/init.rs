use super::*;
use super::panel_state::{GrpcPanel, WsPanel, SioPanel, TrpcPanel, GraphqlPanel, ScriptPanel};
use gpui_component::input::InputState;

impl<E: WebSocketExecutor> RequestPanel<E> {
    pub fn new(window: &mut Window, cx: &mut Context<Self>, response_panel: Entity<ResponsePanel>) -> Self {
        let url = "https://httpbin.org/post".to_string();
        let url_len = url.len();
        let initial_body = "{\n  \"name\": \"Protide\",\n  \"version\": \"0.1.0\"\n}";
        let body_editor = cx.new(|cx| {
            InputState::new(window, cx).multi_line(true).code_editor("json").line_number(true).default_value(initial_body)
        });
        let codegen_editor = cx.new(|cx| {
            InputState::new(window, cx).multi_line(true).line_number(true)
        });
        let import_editor = cx.new(|cx| {
            InputState::new(window, cx).multi_line(true)
        });
        let grpc = GrpcPanel {
            message_editor: cx.new(|cx| {
                InputState::new(window, cx).multi_line(true).code_editor("json").line_number(true).default_value("{}")
            }),
            metadata: vec![KeyValuePair::default()],
            proto_path: None,
            proto_content: String::new(),
            services: Vec::new(),
            service: None,
            methods: Vec::new(),
            method: None,
        };
        let ws = WsPanel {
            state: WsConnectionState::Disconnected,
            messages: WsRingBuffer::default(),
            message_editor: cx.new(|cx| {
                InputState::new(window, cx).multi_line(true).code_editor("json").line_number(true).default_value("{\"type\": \"hello\"}")
            }),
            send_tx: None,
            compose_h: 120.0,
            compose_drag: None,
            scroll: gpui::ScrollHandle::new(),
        };
        let sio = SioPanel {
            state: SioConnectionState::Disconnected,
            messages: SioRingBuffer::default(),
            namespace: "/".to_string(),
            event_name: "message".to_string(),
            want_ack: false,
            next_ack_id: 1,
            payload_editor: cx.new(|cx| {
                InputState::new(window, cx).multi_line(true).code_editor("json").default_value("{}")
            }),
            send_tx: None,
            room_name: String::new(),
            active_rooms: Vec::new(),
        };
        let trpc = TrpcPanel {
            procedure: String::new(),
            params_editor: cx.new(|cx| {
                InputState::new(window, cx).multi_line(true).code_editor("json").line_number(true).default_value("{}")
            }),
            pg_procedures: vec![],
            pg_selected: None,
            pg_loading: false,
            pg_response: None,
            pg_error: None,
            pg_status: None,
            pg_elapsed: None,
            pg_search_input: cx.new(|cx| {
                InputState::new(window, cx).placeholder("Search procedures...")
            }),
            pg_result_viewer: cx.new(|cx| {
                InputState::new(window, cx).multi_line(true).code_editor("json").line_number(true)
            }),
            pg_add_input: cx.new(|cx| {
                InputState::new(window, cx).placeholder("router.name")
            }),
            pg_add_kind: TrpcProcKind::Query,
            pg_sidebar_w: 220.0,
            pg_sidebar_drag: None,
            pg_schema_loading: false,
            pg_schema_error: None,
            pg_import_url_input: cx.new(|cx| {
                InputState::new(window, cx).placeholder("https://api.example.com/trpc")
            }),
            pg_show_import_url: false,
            pg_editing: None,
            pg_edit_input: cx.new(|cx| {
                InputState::new(window, cx).placeholder("router.name")
            }),
            pg_edit_kind: TrpcProcKind::Query,
            pg_editing_group: None,
            pg_group_edit_input: cx.new(|cx| {
                InputState::new(window, cx).placeholder("router")
            }),
        };
        let graphql = GraphqlPanel {
            query_editor: cx.new(|cx| {
                InputState::new(window, cx).multi_line(true).code_editor("graphql").line_number(true).default_value("query {\n  \n}")
            }),
            variables_editor: cx.new(|cx| {
                InputState::new(window, cx).multi_line(true).code_editor("json").line_number(true).default_value("{}")
            }),
            operation_name: String::new(),
            schema: GraphqlSchemaState::Idle,
            schema_search: String::new(),
        };
        let scripts = ScriptPanel {
            pre: String::new(),
            post: String::new(),
            tests: String::new(),
            pre_editor: cx.new(|cx| {
                InputState::new(window, cx).multi_line(true).code_editor("javascript").line_number(true)
            }),
            post_editor: cx.new(|cx| {
                InputState::new(window, cx).multi_line(true).code_editor("javascript").line_number(true)
            }),
            tests_editor: cx.new(|cx| {
                InputState::new(window, cx).multi_line(true).code_editor("javascript").line_number(true)
            }),
            pre_open: true,
            post_open: true,
            tests_open: true,
            pre_h: crate::prefs::get_f32("request.script_pre_h", 160.0),
            post_h: crate::prefs::get_f32("request.script_post_h", 160.0),
            drag_pre: None,
            drag_post: None,
        };
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
            url_undo_stack: std::collections::VecDeque::new(),
            url_redo_stack: std::collections::VecDeque::new(),
            edit_undo_stack: std::collections::VecDeque::new(),
            edit_redo_stack: std::collections::VecDeque::new(),
            skip_blur: false,
            edit_focus: cx.focus_handle(),
            body_focus: cx.focus_handle(),
            explorer_panel: None,
            body_editor,
            variable_extractions: Vec::new(),
            codegen_content: None,
            codegen_language: CodegenLanguage::Curl,
            codegen_editor,
            import_modal_open: false,
            import_text: String::new(),
            import_error: None,
            import_editor,
            request_mode: RequestMode::Http,
            grpc,
            ws,
            sio,
            trpc,
            graphql,
            scripts,
            kv_col_key_w: 150.0,
            kv_col_drag: None,
            current_file: None,
            save_feedback: false,
            custom_method_input: String::new(),
            custom_method_focus: cx.focus_handle(),
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
