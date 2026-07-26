//! Mock server panel UI

mod render;
mod render_form;

use crate::main_window::MainWindow;
use crate::theme;
use gpui::{Context, Entity, WeakEntity, Window, prelude::*};
use gpui_component::input::InputState;
use protide_core::mock_server::{HttpMethod, MockResponse, MockRoute, MockServer};

pub struct MockServerPanel {
    pub(super) server: MockServer,
    pub(super) new_route_method: HttpMethod,
    pub(super) status_input: Entity<InputState>,
    pub(super) mock_path_input: Entity<InputState>,
    pub(super) proxy_path_input: Entity<InputState>,
    pub(super) proxy_target_input: Entity<InputState>,
    pub(super) record_target_input: Entity<InputState>,
    pub(super) main_window: WeakEntity<MainWindow>,
}

impl MockServerPanel {
    pub fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        main_window: WeakEntity<MainWindow>,
    ) -> Self {
        let status_input = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value("200")
                .placeholder("200")
        });
        let mock_path_input = cx.new(|cx| InputState::new(window, cx).placeholder("/api/mock"));
        let proxy_path_input = cx.new(|cx| InputState::new(window, cx).placeholder("/api/*"));
        let proxy_target_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("https://api.example.com"));
        let record_target_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("https://api.example.com"));
        Self {
            server: MockServer::new(8080),
            new_route_method: HttpMethod::Get,
            status_input,
            mock_path_input,
            proxy_path_input,
            proxy_target_input,
            record_target_input,
            main_window,
        }
    }

    pub(super) fn toggle_server(&mut self, cx: &mut Context<Self>) {
        if self.server.is_running() {
            self.server.stop();
        } else if let Err(msg) = self.server.start()
            && let Some(win) = self.main_window.upgrade()
        {
            win.update(cx, |win, cx| {
                win.show_modal("Failed to Start Mock Server", msg, cx)
            });
        }
        cx.notify();
    }

    pub(super) fn add_route(&mut self, cx: &mut Context<Self>) {
        let status = self
            .status_input
            .read(cx)
            .value()
            .to_string()
            .trim()
            .parse::<u16>()
            .unwrap_or(200);
        let raw_path = self.mock_path_input.read(cx).value().to_string();
        let path = if raw_path.trim().is_empty() {
            "/api/mock".to_string()
        } else {
            raw_path.trim().to_string()
        };
        let response = MockResponse::new(status, r#"{"message":"mock response"}"#)
            .with_header("Content-Type", "application/json");
        let route = MockRoute::new(self.new_route_method, path, response);
        self.server.add_route(route);
        cx.notify();
    }

    pub(super) fn add_proxy_route(&mut self, cx: &mut Context<Self>) {
        let raw_path = self.proxy_path_input.read(cx).value().to_string();
        let raw_target = self.proxy_target_input.read(cx).value().to_string();
        let path = if raw_path.trim().is_empty() {
            "/api/*".to_string()
        } else {
            raw_path.trim().to_string()
        };
        let target = if raw_target.trim().is_empty() {
            "https://api.example.com".to_string()
        } else {
            raw_target.trim().to_string()
        };
        let route = MockRoute::proxy(self.new_route_method, &path, &target);
        self.server.add_route(route);
        let msg = format!(
            "Proxy route added: {} {} → {}",
            self.new_route_method.as_str(),
            path,
            target
        );
        if let Some(win) = self.main_window.upgrade() {
            win.update(cx, |win, cx| win.show_modal("Proxy Route Added", msg, cx));
        }
        cx.notify();
    }

    pub(super) fn remove_route(&mut self, index: usize, cx: &mut Context<Self>) {
        self.server.remove_route(index);
        cx.notify();
    }

    pub(super) fn set_method(&mut self, method: HttpMethod, cx: &mut Context<Self>) {
        self.new_route_method = method;
        cx.notify();
    }

    pub(super) fn toggle_record_mode(&mut self, cx: &mut Context<Self>) {
        let recording = self.server.is_recording();
        if recording {
            self.server.set_record_mode(false, None);
        } else {
            let target = self
                .record_target_input
                .read(cx)
                .value()
                .to_string()
                .trim()
                .to_string();
            let target = if target.is_empty() {
                None
            } else {
                Some(target)
            };
            self.server.set_record_mode(true, target);
            if !self.server.is_running()
                && let Err(msg) = self.server.start()
                && let Some(win) = self.main_window.upgrade()
            {
                win.update(cx, |win, cx| {
                    win.show_modal("Failed to Start Mock Server", msg, cx)
                });
            }
        }
        cx.notify();
    }

    pub(super) fn import_recorded(&mut self, cx: &mut Context<Self>) {
        let captured = self.server.drain_recorded();
        let count = captured.len();
        for route in captured {
            self.server.add_route(route);
        }
        if count > 0 {
            let msg = format!(
                "Imported {} recorded route{}.",
                count,
                if count == 1 { "" } else { "s" }
            );
            if let Some(win) = self.main_window.upgrade() {
                win.update(cx, |win, cx| win.show_modal("Routes Imported", msg, cx));
            }
        }
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::init_theme;
    use gpui::{TestAppContext, VisualTestContext};

    /// The panel with no `MainWindow` behind it: its `upgrade()` fails, so the
    /// modal side-effects are skipped and only the route state is exercised.
    /// Nothing here starts the server, so no port is ever bound.
    fn panel(cx: &mut TestAppContext) -> (Entity<MockServerPanel>, &mut VisualTestContext) {
        init_theme(cx);
        cx.add_window_view(|window, cx| MockServerPanel::new(window, cx, WeakEntity::new_invalid()))
    }

    fn set(cx: &mut VisualTestContext, input: &Entity<InputState>, value: &str) {
        cx.update_window_entity(input, |state, window, cx| {
            state.set_value(value, window, cx)
        });
    }

    #[gpui::test]
    async fn a_new_panel_has_no_routes_and_is_not_running(cx: &mut TestAppContext) {
        let (p, cx) = panel(cx);
        p.read_with(cx, |p, _| {
            assert!(p.server.routes().is_empty());
            assert!(!p.server.is_running());
            assert!(!p.server.is_recording());
            assert_eq!(p.new_route_method, HttpMethod::Get);
        });
    }

    #[gpui::test]
    async fn a_route_uses_the_typed_path_status_and_method(cx: &mut TestAppContext) {
        let (p, cx) = panel(cx);
        let (path_input, status_input) = p.read_with(cx, |p, _| {
            (p.mock_path_input.clone(), p.status_input.clone())
        });
        set(cx, &path_input, "/users/42");
        set(cx, &status_input, "404");
        p.update(cx, |p, cx| {
            p.set_method(HttpMethod::Post, cx);
            p.add_route(cx);
        });
        p.read_with(cx, |p, _| {
            let routes = p.server.routes();
            assert_eq!(routes.len(), 1);
            assert_eq!(routes[0].path, "/users/42");
            assert_eq!(routes[0].response.status, 404);
            assert_eq!(routes[0].method, HttpMethod::Post);
        });
    }

    #[gpui::test]
    async fn a_blank_or_whitespace_path_falls_back_to_the_placeholder(cx: &mut TestAppContext) {
        let (p, cx) = panel(cx);
        let input = p.read_with(cx, |p, _| p.mock_path_input.clone());
        for typed in ["", "   ", "\t"] {
            set(cx, &input, typed);
            p.update(cx, |p, cx| p.add_route(cx));
        }
        p.read_with(cx, |p, _| {
            let routes = p.server.routes();
            assert_eq!(routes.len(), 3);
            for r in &routes {
                assert_eq!(
                    r.path, "/api/mock",
                    "a blank path must not create an empty route"
                );
            }
        });
    }

    #[gpui::test]
    async fn a_typed_path_is_trimmed(cx: &mut TestAppContext) {
        let (p, cx) = panel(cx);
        let input = p.read_with(cx, |p, _| p.mock_path_input.clone());
        set(cx, &input, "  /padded  ");
        p.update(cx, |p, cx| p.add_route(cx));
        p.read_with(cx, |p, _| assert_eq!(p.server.routes()[0].path, "/padded"));
    }

    #[gpui::test]
    async fn an_unparseable_status_falls_back_to_200(cx: &mut TestAppContext) {
        // The status field is free text; "abc", an empty box or an out-of-range
        // number must not take down the panel or persist a nonsense status.
        let (p, cx) = panel(cx);
        let input = p.read_with(cx, |p, _| p.status_input.clone());
        for typed in ["", "abc", "99999", "-1", "2.5", "20 0"] {
            set(cx, &input, typed);
            p.update(cx, |p, cx| p.add_route(cx));
        }
        p.read_with(cx, |p, _| {
            for r in p.server.routes() {
                assert_eq!(
                    r.response.status, 200,
                    "bad status text must default to 200"
                );
            }
        });
    }

    #[gpui::test]
    async fn a_proxy_route_uses_the_typed_path_and_target(cx: &mut TestAppContext) {
        let (p, cx) = panel(cx);
        let (path, target) = p.read_with(cx, |p, _| {
            (p.proxy_path_input.clone(), p.proxy_target_input.clone())
        });
        set(cx, &path, " /v1/* ");
        set(cx, &target, " https://upstream.test ");
        p.update(cx, |p, cx| p.add_proxy_route(cx));
        p.read_with(cx, |p, _| {
            let r = &p.server.routes()[0];
            assert!(r.is_proxy());
            assert_eq!(r.path, "/v1/*");
        });
    }

    #[gpui::test]
    async fn a_blank_proxy_form_falls_back_to_its_placeholders(cx: &mut TestAppContext) {
        let (p, cx) = panel(cx);
        p.update(cx, |p, cx| p.add_proxy_route(cx));
        p.read_with(cx, |p, _| {
            let r = &p.server.routes()[0];
            assert!(r.is_proxy());
            assert_eq!(r.path, "/api/*");
        });
    }

    #[gpui::test]
    async fn removing_a_route_that_does_not_exist_is_a_no_op(cx: &mut TestAppContext) {
        let (p, cx) = panel(cx);
        p.update(cx, |p, cx| {
            p.add_route(cx);
            p.remove_route(99, cx);
            p.remove_route(usize::MAX, cx);
            assert_eq!(
                p.server.routes().len(),
                1,
                "a bad index must not drop a route"
            );
            p.remove_route(0, cx);
            assert!(p.server.routes().is_empty());
        });
    }

    #[gpui::test]
    async fn importing_with_nothing_recorded_adds_no_routes(cx: &mut TestAppContext) {
        let (p, cx) = panel(cx);
        p.update(cx, |p, cx| {
            p.import_recorded(cx);
            assert!(p.server.routes().is_empty());
        });
    }

    #[gpui::test]
    async fn turning_record_mode_off_leaves_the_server_alone(cx: &mut TestAppContext) {
        // Only the *enabling* branch may start the server; disabling must not.
        let (p, cx) = panel(cx);
        p.update(cx, |p, cx| {
            p.server
                .set_record_mode(true, Some("https://upstream.test".into()));
            p.toggle_record_mode(cx);
            assert!(!p.server.is_recording());
            assert!(
                !p.server.is_running(),
                "disabling recording must not start a server"
            );
        });
    }
}
