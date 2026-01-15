//! Main application window

use gpui::{
    div, prelude::*, px, Entity, InteractiveElement,
    IntoElement, ParentElement, Render, Styled, Window, Context, App,
};

use crate::theme;
use super::panels::{ExplorerPanel, MockServerPanel, RequestPanel, ResponsePanel};

/// Main window containing the application layout
pub struct MainWindow {
    explorer: Entity<ExplorerPanel>,
    request_panel: Entity<RequestPanel>,
    response_panel: Entity<ResponsePanel>,
    mock_server_panel: Entity<MockServerPanel>,
    show_mock_server: bool,
}

impl MainWindow {
    pub fn build(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let explorer = cx.new(|cx| ExplorerPanel::new(cx));
        let response_panel = cx.new(|cx| ResponsePanel::new(cx));
        let response_panel_clone = response_panel.clone();
        let request_panel = cx.new(|cx| RequestPanel::new(cx, response_panel_clone));
        let mock_server_panel = cx.new(|cx| MockServerPanel::new(cx));

        // Connect explorer to request panel for history loading
        let request_panel_clone = request_panel.clone();
        explorer.update(cx, |explorer, cx| {
            explorer.set_request_panel(request_panel_clone, cx);
        });

        // Connect request panel to explorer for environment variable substitution
        let explorer_clone = explorer.clone();
        request_panel.update(cx, |panel, cx| {
            panel.set_explorer_panel(explorer_clone, cx);
        });

        Self {
            explorer,
            request_panel,
            response_panel,
            mock_server_panel,
            show_mock_server: false,
        }
    }

    fn toggle_mock_server(&mut self, cx: &mut Context<Self>) {
        self.show_mock_server = !self.show_mock_server;
        cx.notify();
    }
}

impl Render for MainWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.colors.bg_primary)
            .text_color(theme.colors.text_primary)
            // Removed track_focus to allow child panels to receive key events
            // Title bar with window controls
            .child(self.render_title_bar(cx))
            // Main content
            .child(
                div()
                    .flex_1()
                    .flex()
                    .overflow_hidden()
                    // Sidebar (Explorer)
                    .child(
                        div()
                            .w(px(250.0))
                            .h_full()
                            .border_r_1()
                            .border_color(theme.colors.border)
                            .bg(theme.colors.bg_secondary)
                            .child(self.explorer.clone())
                    )
                    // Main content area
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .overflow_hidden()
                            // Request panel (top) - 45% of space
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .min_h(px(200.0))
                                    .h(gpui::relative(0.45))
                                    .w_full()
                                    .border_b_1()
                                    .border_color(theme.colors.border)
                                    .child(self.request_panel.clone())
                            )
                            // Response panel (bottom) - 55% of space
                            .child(
                                div()
                                    .flex_1()
                                    .min_h(px(150.0))
                                    .w_full()
                                    .overflow_hidden()
                                    .child(self.response_panel.clone())
                            )
                    )
                    // Mock Server panel (right sidebar, optional)
                    .when(self.show_mock_server, |el| {
                        el.child(
                            div()
                                .w(px(320.0))
                                .h_full()
                                .child(self.mock_server_panel.clone())
                        )
                    })
            )
    }
}

impl MainWindow {
    fn render_title_bar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let show_mock = self.show_mock_server;

        div()
            .id("titlebar")
            .h(px(38.0))
            .w_full()
            .flex()
            .items_center()
            .justify_between()
            .bg(theme.colors.bg_primary)
            .border_b_1()
            .border_color(theme.colors.border)
            // Left side - Logo and title (draggable area)
            .child(
                div()
                    .id("titlebar-drag")
                    .flex_1()
                    .h_full()
                    .flex()
                    .items_center()
                    .px(px(12.0))
                    .cursor_pointer()
                    .on_mouse_down(gpui::MouseButton::Left, |_, window, _cx: &mut App| {
                        window.start_window_move();
                    })
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(10.0))
                            // Logo - gradient-style badge
                            .child(
                                div()
                                    .size(px(22.0))
                                    .rounded(px(6.0))
                                    .bg(theme.colors.accent)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(
                                        div()
                                            .text_size(px(11.0))
                                            .font_weight(gpui::FontWeight::BOLD)
                                            .text_color(gpui::white())
                                            .child("A")
                                    )
                            )
                            // Title
                            .child(
                                div()
                                    .text_size(px(13.0))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.colors.text_primary)
                                    .child("API Dash")
                            )
                    )
            )
            // Mock Server toggle button
            .child(
                div()
                    .id("btn-mock-server")
                    .h(px(26.0))
                    .px(px(10.0))
                    .mr(px(8.0))
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .rounded(px(4.0))
                    .cursor_pointer()
                    .bg(if show_mock { theme.colors.accent } else { theme.colors.bg_tertiary })
                    .hover(|s| s.bg(if show_mock { theme.colors.accent } else { theme.colors.bg_elevated }))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.toggle_mock_server(cx);
                    }))
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(if show_mock { gpui::white() } else { theme.colors.text_secondary })
                            .child("Mock Server")
                    )
            )
            // Right side - Window controls
            .child(
                div()
                    .flex()
                    .items_center()
                    .h_full()
                    .gap(px(1.0))
                    // Minimize button
                    .child(
                        div()
                            .id("btn-minimize")
                            .w(px(44.0))
                            .h_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.colors.bg_tertiary))
                            .active(|s| s.bg(theme.colors.bg_elevated))
                            .on_click(|_, window, _cx: &mut App| {
                                window.minimize_window();
                            })
                            .child(
                                div()
                                    .w(px(10.0))
                                    .h(px(1.0))
                                    .bg(theme.colors.text_muted)
                            )
                    )
                    // Maximize button
                    .child(
                        div()
                            .id("btn-maximize")
                            .w(px(44.0))
                            .h_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.colors.bg_tertiary))
                            .active(|s| s.bg(theme.colors.bg_elevated))
                            .on_click(|_, window, _cx: &mut App| {
                                window.toggle_fullscreen();
                            })
                            .child(
                                div()
                                    .size(px(9.0))
                                    .border_1()
                                    .border_color(theme.colors.text_muted)
                                    .rounded(px(1.0))
                            )
                    )
                    // Close button
                    .child(
                        div()
                            .id("btn-close")
                            .w(px(44.0))
                            .h_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .text_color(theme.colors.text_muted)
                            .hover(|s| s.bg(gpui::rgb(0xc42b1c)).text_color(gpui::white()))
                            .on_click(|_, _window, cx: &mut App| {
                                cx.quit();
                            })
                            .child(
                                div()
                                    .text_size(px(14.0))
                                    .child("✕")
                            )
                    )
            )
    }
}
