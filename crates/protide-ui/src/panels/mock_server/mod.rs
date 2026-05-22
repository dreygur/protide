//! Mock server panel UI

mod render;
mod render_form;

use gpui::{Context, Entity, FocusHandle, WeakEntity, Window, prelude::*};
use protide_core::mock_server::{HttpMethod, MockResponse, MockRoute, MockServer};
use crate::theme;
use crate::components::modal::ModalState;
use gpui_component::input::InputState;
use crate::main_window::MainWindow;

pub struct MockServerPanel {
    pub(super) server: MockServer,
    #[allow(dead_code)]
    pub(super) focus: FocusHandle,
    pub(super) new_route_method: HttpMethod,
    pub(super) status_input: Entity<InputState>,
    pub(super) proxy_path_input: Entity<InputState>,
    pub(super) proxy_target_input: Entity<InputState>,
    pub(super) record_target_input: Entity<InputState>,
    pub(super) main_window: WeakEntity<MainWindow>,
}

impl MockServerPanel {
    pub fn new(window: &mut Window, cx: &mut Context<Self>, main_window: WeakEntity<MainWindow>) -> Self {
        let status_input = cx.new(|cx| InputState::new(window, cx).default_value("200").placeholder("200"));
        let proxy_path_input = cx.new(|cx| InputState::new(window, cx).placeholder("/api/*"));
        let proxy_target_input = cx.new(|cx| InputState::new(window, cx).placeholder("https://api.example.com"));
        let record_target_input = cx.new(|cx| InputState::new(window, cx).placeholder("https://api.example.com"));
        Self {
            server: MockServer::new(8080),
            focus: cx.focus_handle(),
            new_route_method: HttpMethod::Get,
            status_input,
            proxy_path_input,
            proxy_target_input,
            record_target_input,
            main_window,
        }
    }

    pub(super) fn toggle_server(&mut self, cx: &mut Context<Self>) {
        if self.server.is_running() {
            self.server.stop();
        } else {
            let _ = self.server.start();
        }
        cx.notify();
    }

    pub(super) fn add_route(&mut self, cx: &mut Context<Self>) {
        let status = self.status_input.read(cx).value().to_string().trim().parse::<u16>().unwrap_or(200);
        let response = MockResponse::new(status, r#"{"message":"mock response"}"#)
            .with_header("Content-Type", "application/json");
        let route = MockRoute::new(self.new_route_method, "/api/mock", response);
        self.server.add_route(route);
        cx.notify();
    }

    pub(super) fn add_proxy_route(&mut self, cx: &mut Context<Self>) {
        let raw_path = self.proxy_path_input.read(cx).value().to_string();
        let raw_target = self.proxy_target_input.read(cx).value().to_string();
        let path = if raw_path.trim().is_empty() { "/api/*".to_string() } else { raw_path.trim().to_string() };
        let target = if raw_target.trim().is_empty() { "https://api.example.com".to_string() } else { raw_target.trim().to_string() };
        let route = MockRoute::proxy(self.new_route_method, &path, &target);
        self.server.add_route(route);
        let modal = ModalState::info(
            "Proxy Route Added",
            format!("Proxy route added: {} {} → {}", self.new_route_method.as_str(), path, target),
        );
        if let Some(win) = self.main_window.upgrade() {
            win.update(cx, |win, cx| win.show_modal(modal, cx));
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
            let target = self.record_target_input.read(cx).value().to_string().trim().to_string();
            let target = if target.is_empty() { None } else { Some(target) };
            self.server.set_record_mode(true, target);
            if !self.server.is_running() {
                let _ = self.server.start();
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
            let modal = ModalState::info(
                "Routes Imported",
                format!("Imported {} recorded route{}.", count, if count == 1 { "" } else { "s" }),
            );
            if let Some(win) = self.main_window.upgrade() {
                win.update(cx, |win, cx| win.show_modal(modal, cx));
            }
        }
        cx.notify();
    }
}
