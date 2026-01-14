//! Main application window

use gpui::{
    div, prelude::*, px, Entity, FocusHandle, InteractiveElement,
    IntoElement, ParentElement, Render, Styled, Window, Context, App,
};

use crate::theme;
use super::panels::{ExplorerPanel, RequestPanel, ResponsePanel};

/// Main window containing the application layout
pub struct MainWindow {
    focus_handle: FocusHandle,
    explorer: Entity<ExplorerPanel>,
    request_panel: Entity<RequestPanel>,
    response_panel: Entity<ResponsePanel>,
}

impl MainWindow {
    pub fn build(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();

        let explorer = cx.new(|cx| ExplorerPanel::new(cx));
        let response_panel = cx.new(|cx| ResponsePanel::new(cx));
        let response_panel_clone = response_panel.clone();
        let request_panel = cx.new(|cx| RequestPanel::new(cx, response_panel_clone));

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
            focus_handle,
            explorer,
            request_panel,
            response_panel,
        }
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
            .track_focus(&self.focus_handle)
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
                            // Request panel (top)
                            .child(
                                div()
                                    .h(px(350.0))
                                    .w_full()
                                    .border_b_1()
                                    .border_color(theme.colors.border)
                                    .child(self.request_panel.clone())
                            )
                            // Response panel (bottom)
                            .child(
                                div()
                                    .flex_1()
                                    .w_full()
                                    .overflow_hidden()
                                    .child(self.response_panel.clone())
                            )
                    )
            )
    }
}

impl MainWindow {
    fn render_title_bar(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);

        div()
            .id("titlebar")
            .h(px(40.0))
            .w_full()
            .flex()
            .items_center()
            .justify_between()
            .bg(theme.colors.bg_secondary)
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
                    .px(px(16.0))
                    .cursor_pointer()
                    .on_mouse_down(gpui::MouseButton::Left, |_, window, _cx: &mut App| {
                        window.start_window_move();
                    })
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            // Logo
                            .child(
                                div()
                                    .size(px(24.0))
                                    .rounded(px(4.0))
                                    .bg(theme.colors.accent)
                            )
                            // Title
                            .child(
                                div()
                                    .text_size(px(14.0))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .child("API Dash")
                            )
                    )
            )
            // Right side - Window controls
            .child(
                div()
                    .flex()
                    .items_center()
                    .h_full()
                    // Minimize button
                    .child(
                        div()
                            .id("btn-minimize")
                            .w(px(46.0))
                            .h_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.colors.bg_tertiary))
                            .on_click(|_, window, _cx: &mut App| {
                                window.minimize_window();
                            })
                            .child(
                                div()
                                    .w(px(10.0))
                                    .h(px(1.0))
                                    .bg(theme.colors.text_secondary)
                            )
                    )
                    // Maximize button
                    .child(
                        div()
                            .id("btn-maximize")
                            .w(px(46.0))
                            .h_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.colors.bg_tertiary))
                            .on_click(|_, window, _cx: &mut App| {
                                window.toggle_fullscreen();
                            })
                            .child(
                                div()
                                    .size(px(10.0))
                                    .border_1()
                                    .border_color(theme.colors.text_secondary)
                            )
                    )
                    // Close button
                    .child(
                        div()
                            .id("btn-close")
                            .w(px(46.0))
                            .h_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .hover(|s| s.bg(gpui::rgb(0xe81123)))
                            .on_click(|_, _window, cx: &mut App| {
                                cx.quit();
                            })
                            .child(
                                div()
                                    .text_size(px(16.0))
                                    .text_color(theme.colors.text_secondary)
                                    .child("×")
                            )
                    )
            )
    }
}
