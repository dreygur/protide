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
}
