//! Main application window

use std::path::PathBuf;

use gpui::{
    div, prelude::*, px, Entity, FontWeight, InteractiveElement,
    IntoElement, ParentElement, Render, Styled, WeakEntity, Window, Context, App,
    MouseButton,
};

use crate::theme;
use crate::ui::components::icons::{icon, ICON_MD, ICON_MENU, ICON_MINIMIZE, ICON_MAXIMIZE, ICON_WINDOW_CLOSE};
use crate::ui::components::modal::{ModalKind, ModalState, render_modal_shell};
use super::panels::{ExplorerPanel, MockServerPanel, RequestPanel, ResponsePanel};

/// Pending action for confirm modals
#[derive(Clone, Debug, Default)]
enum ModalPending {
    #[default]
    None,
    ExplorerDelete(PathBuf),
}

/// Main window containing the application layout
pub struct MainWindow {
    explorer: Entity<ExplorerPanel>,
    request_panel: Entity<RequestPanel>,
    response_panel: Entity<ResponsePanel>,
    mock_server_panel: Entity<MockServerPanel>,
    show_mock_server: bool,
    sidebar_collapsed: bool,
    sidebar_width: f32,
    request_height: f32,
    mock_server_width: f32,
    drag_sidebar: Option<(f32, f32)>,      // (start_mouse_x, start_sidebar_width)
    drag_response: Option<(f32, f32)>,     // (start_mouse_y, start_request_height)
    drag_mock_server: Option<(f32, f32)>,  // (start_mouse_x, start_mock_server_width)
    modal: Option<ModalState>,
    modal_pending: ModalPending,
}

impl MainWindow {
    pub fn build(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let main_window_weak: WeakEntity<MainWindow> = cx.entity().downgrade();
        let explorer = cx.new(|cx| ExplorerPanel::new(cx, main_window_weak.clone()));
        let response_panel = cx.new(|cx| ResponsePanel::new(cx));
        let response_panel_clone = response_panel.clone();
        let request_panel = cx.new(|cx| RequestPanel::new(cx, response_panel_clone));
        let mock_server_panel = cx.new(|cx| MockServerPanel::new(cx, main_window_weak));

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
            sidebar_collapsed: false,
            sidebar_width: 250.0,
            request_height: 320.0,
            mock_server_width: 320.0,
            drag_sidebar: None,
            drag_response: None,
            drag_mock_server: None,
            modal: None,
            modal_pending: ModalPending::None,
        }
    }

    pub fn toggle_sidebar(&mut self, cx: &mut Context<Self>) {
        self.sidebar_collapsed = !self.sidebar_collapsed;
        cx.notify();
    }

    fn toggle_mock_server(&mut self, cx: &mut Context<Self>) {
        self.show_mock_server = !self.show_mock_server;
        cx.notify();
    }

    pub fn show_modal(&mut self, state: ModalState, cx: &mut Context<Self>) {
        self.modal = Some(state);
        self.modal_pending = ModalPending::None;
        cx.notify();
    }

    pub fn show_confirm_delete(&mut self, state: ModalState, path: PathBuf, cx: &mut Context<Self>) {
        self.modal = Some(state);
        self.modal_pending = ModalPending::ExplorerDelete(path);
        cx.notify();
    }

    fn dismiss_modal(&mut self, cx: &mut Context<Self>) {
        self.modal = None;
        self.modal_pending = ModalPending::None;
        cx.notify();
    }

    fn confirm_modal_action(&mut self, cx: &mut Context<Self>) {
        let pending = std::mem::replace(&mut self.modal_pending, ModalPending::None);
        self.modal = None;
        match pending {
            ModalPending::ExplorerDelete(path) => {
                self.explorer.update(cx, |panel, cx| panel.execute_delete(path, cx));
            }
            ModalPending::None => {}
        }
        cx.notify();
    }
}

impl Render for MainWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let is_dragging = self.drag_sidebar.is_some() || self.drag_response.is_some() || self.drag_mock_server.is_some();
        let is_col_drag = self.drag_sidebar.is_some() || self.drag_mock_server.is_some();

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.colors.bg_primary)
            .text_color(theme.colors.text_primary)
            .child(self.render_title_bar(cx))
            .child(
                div()
                    .flex_1()
                    .flex()
                    .overflow_hidden()
                    // Collapsed sidebar strip
                    .when(self.sidebar_collapsed, |el| {
                        el.child(
                            div()
                                .id("sidebar-collapsed-strip")
                                .w(px(32.0))
                                .h_full()
                                .flex_shrink_0()
                                .bg(theme.colors.bg_secondary)
                                .border_r_1()
                                .border_color(theme.colors.border)
                                .flex()
                                .flex_col()
                                .items_center()
                                .pt(px(12.0))
                                .cursor_pointer()
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.toggle_sidebar(cx);
                                }))
                                .child(icon(ICON_MENU, ICON_MD, theme.colors.text_muted))
                        )
                    })
                    // Full sidebar
                    .when(!self.sidebar_collapsed, |el| {
                        el
                            .child(
                                div()
                                    .w(px(self.sidebar_width))
                                    .h_full()
                                    .flex_shrink_0()
                                    .bg(theme.colors.bg_secondary)
                                    .overflow_hidden()
                                    .child(self.explorer.clone())
                            )
                            // Sidebar resize handle
                            .child(
                                div()
                                    .id("sidebar-resize-handle")
                                    .w(px(4.0))
                                    .h_full()
                                    .flex_shrink_0()
                                    .border_l_1()
                                    .border_color(theme.colors.border)
                                    .cursor_col_resize()
                                    .hover(|s| s.bg(theme.colors.accent.opacity(0.25)))
                                    .on_mouse_down(MouseButton::Left, cx.listener(|this, event: &gpui::MouseDownEvent, _window, _cx| {
                                        this.drag_sidebar = Some((f32::from(event.position.x), this.sidebar_width));
                                    }))
                            )
                    })
                    // Main content area
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .overflow_hidden()
                            // Request panel
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .flex_shrink_0()
                                    .min_h(px(150.0))
                                    .h(px(self.request_height))
                                    .w_full()
                                    .overflow_hidden()
                                    .child(self.request_panel.clone())
                            )
                            // Response resize handle
                            .child(
                                div()
                                    .id("response-resize-handle")
                                    .w_full()
                                    .h(px(4.0))
                                    .flex_shrink_0()
                                    .border_t_1()
                                    .border_color(theme.colors.border)
                                    .cursor_row_resize()
                                    .hover(|s| s.bg(theme.colors.accent.opacity(0.25)))
                                    .on_mouse_down(MouseButton::Left, cx.listener(|this, event: &gpui::MouseDownEvent, _window, _cx| {
                                        this.drag_response = Some((f32::from(event.position.y), this.request_height));
                                    }))
                            )
                            // Response panel
                            .child(
                                div()
                                    .flex_1()
                                    .min_h(px(150.0))
                                    .w_full()
                                    .overflow_hidden()
                                    .child(self.response_panel.clone())
                            )
                    )
                    // Mock Server panel (optional right sidebar)
                    .when(self.show_mock_server, |el| {
                        el
                        // Mock server resize handle (left edge)
                        .child(
                            div()
                                .id("mock-server-resize-handle")
                                .w(px(4.0))
                                .h_full()
                                .flex_shrink_0()
                                .border_r_1()
                                .border_color(theme.colors.border)
                                .cursor_col_resize()
                                .hover(|s| s.bg(theme.colors.accent.opacity(0.25)))
                                .on_mouse_down(MouseButton::Left, cx.listener(|this, event: &gpui::MouseDownEvent, _window, _cx| {
                                    this.drag_mock_server = Some((f32::from(event.position.x), this.mock_server_width));
                                }))
                        )
                        .child(
                            div()
                                .w(px(self.mock_server_width))
                                .h_full()
                                .flex_shrink_0()
                                .overflow_hidden()
                                .child(self.mock_server_panel.clone())
                        )
                    })
                    // Drag overlay — captures mouse during resize, must be last child
                    .when(is_dragging, |el| {
                        el.child(
                            div()
                                .id("resize-drag-overlay")
                                .absolute()
                                .top_0()
                                .left_0()
                                .w_full()
                                .h_full()
                                .when(is_col_drag, |el| el.cursor_col_resize())
                                .when(!is_col_drag, |el| el.cursor_row_resize())
                                .on_mouse_move(cx.listener(|this, event: &gpui::MouseMoveEvent, _window, cx| {
                                    let mouse_x = f32::from(event.position.x);
                                    let mouse_y = f32::from(event.position.y);
                                    if let Some((start_x, start_w)) = this.drag_sidebar {
                                        this.sidebar_width = (start_w + mouse_x - start_x).max(150.0).min(600.0);
                                        cx.notify();
                                    }
                                    if let Some((start_y, start_h)) = this.drag_response {
                                        this.request_height = (start_h + mouse_y - start_y).max(150.0).min(800.0);
                                        cx.notify();
                                    }
                                    if let Some((start_x, start_w)) = this.drag_mock_server {
                                        // dragging left edge: moving left increases width
                                        this.mock_server_width = (start_w - (mouse_x - start_x)).max(200.0).min(700.0);
                                        cx.notify();
                                    }
                                }))
                                .on_mouse_up(MouseButton::Left, cx.listener(|this, _, _window, cx| {
                                    this.drag_sidebar = None;
                                    this.drag_response = None;
                                    this.drag_mock_server = None;
                                    cx.notify();
                                }))
                        )
                    })
            )
            .child(self.render_status_bar(cx))
            // Full-window modal overlay (always on top)
            .when_some(self.modal.clone(), |el, modal| {
                let theme = theme::current(cx);
                let is_confirm = modal.kind == ModalKind::Confirm;
                let buttons = if is_confirm {
                    div()
                        .flex().justify_end().gap(px(8.0)).mt(px(4.0))
                        .child(
                            div()
                                .id("modal-cancel")
                                .px(px(20.0)).py(px(8.0))
                                .bg(theme.colors.bg_tertiary)
                                .border_1()
                                .border_color(theme.colors.border)
                                .text_color(theme.colors.text_secondary)
                                .text_size(px(12.0))
                                .cursor_pointer()
                                .hover(|s| s.bg(theme.colors.bg_elevated))
                                .on_click(cx.listener(|this, _, _, cx| this.dismiss_modal(cx)))
                                .child("Cancel")
                        )
                        .child(
                            div()
                                .id("modal-confirm")
                                .px(px(20.0)).py(px(8.0))
                                .bg(theme.colors.error)
                                .text_color(gpui::white())
                                .text_size(px(12.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .cursor_pointer()
                                .hover(|s| s.opacity(0.85))
                                .on_click(cx.listener(|this, _, _, cx| this.confirm_modal_action(cx)))
                                .child("Delete")
                        )
                        .into_any_element()
                } else {
                    div()
                        .flex().justify_end().mt(px(4.0))
                        .child(
                            div()
                                .id("modal-ok")
                                .px(px(24.0)).py(px(8.0))
                                .bg(theme.colors.accent)
                                .text_color(theme.colors.bg_primary)
                                .text_size(px(12.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .cursor_pointer()
                                .hover(|s| s.bg(theme.colors.accent_hover))
                                .on_click(cx.listener(|this, _, _, cx| this.dismiss_modal(cx)))
                                .child("OK")
                        )
                        .into_any_element()
                };
                el.child(render_modal_shell(&modal, &theme, buttons))
            })
    }
}

impl MainWindow {
    fn render_title_bar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let show_mock = self.show_mock_server;

        div()
            .id("titlebar")
            .h(px(36.0))
            .w_full()
            .flex()
            .items_center()
            .bg(theme.colors.bg_primary)
            .border_b_1()
            .border_color(theme.colors.border)
            // Logo + title (draggable)
            .child(
                div()
                    .id("titlebar-drag")
                    .flex()
                    .items_center()
                    .gap(px(7.0))
                    .px(px(8.0))
                    .h_full()
                    .cursor_pointer()
                    .on_mouse_down(gpui::MouseButton::Left, |_, window, _cx: &mut App| {
                        window.start_window_move();
                    })
                    // Logo badge
                    .child(
                        div()
                            .size(px(18.0))
                            .bg(theme.colors.accent)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .text_color(gpui::rgb(0x00082b))
                                    .child("A")
                            )
                    )
                    // Title
                    .child(
                        div()
                            .text_size(px(12.0))
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(theme.colors.text_primary)
                            .child("Protide")
                    )
            )
            // Drag region (fills remaining space)
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .on_mouse_down(gpui::MouseButton::Left, |_, window, _cx: &mut App| {
                        window.start_window_move();
                    })
            )
            // Mock server toggle
            .child(
                div()
                    .id("btn-mock-server")
                    .h(px(22.0))
                    .px(px(8.0))
                    .mr(px(6.0))
                    .flex()
                    .items_center()
                    .cursor_pointer()
                    .bg(if show_mock {
                        theme.colors.accent.opacity(0.15)
                    } else {
                        theme.colors.bg_elevated
                    })
                    .border_1()
                    .border_color(if show_mock {
                        theme.colors.accent.opacity(0.4)
                    } else {
                        theme.colors.border
                    })
                    .hover(|s| s.border_color(theme.colors.accent.opacity(0.5)))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.toggle_mock_server(cx);
                    }))
                    .child(
                        div()
                            .text_size(px(10.0))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(if show_mock {
                                theme.colors.accent
                            } else {
                                theme.colors.text_secondary
                            })
                            .child("Mock Server")
                    )
            )
            // Window controls
            .child(
                div()
                    .flex()
                    .items_center()
                    .h_full()
                    .border_l_1()
                    .border_color(theme.colors.border)
                    .child(
                        div()
                            .id("btn-minimize")
                            .w(px(36.0))
                            .h_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .text_color(theme.colors.text_secondary)
                            .hover(|s| s.bg(theme.colors.bg_elevated))
                            .on_click(|_, window, _cx: &mut App| {
                                window.minimize_window();
                            })
                            .child(icon(ICON_MINIMIZE, 12.0, theme.colors.text_secondary))
                    )
                    .child(
                        div()
                            .id("btn-maximize")
                            .w(px(36.0))
                            .h_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .text_color(theme.colors.text_secondary)
                            .hover(|s| s.bg(theme.colors.bg_elevated))
                            .on_click(|_, window, _cx: &mut App| {
                                window.toggle_fullscreen();
                            })
                            .child(icon(ICON_MAXIMIZE, 12.0, theme.colors.text_secondary))
                    )
                    .child(
                        div()
                            .id("btn-close")
                            .w(px(36.0))
                            .h_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .text_color(theme.colors.text_secondary)
                            .hover(|s| s.bg(gpui::rgb(0xc42b1c)).text_color(gpui::white()))
                            .on_click(|_, _window, cx: &mut App| {
                                cx.quit();
                            })
                            .child(icon(ICON_WINDOW_CLOSE, 12.0, theme.colors.text_secondary))
                    )
            )
    }

    fn render_status_bar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);

        // Read protocol from request panel
        let protocol = self.request_panel.read(cx).mode_label();
        let protocol_color = theme.method_color(protocol);

        // Read last response summary
        let response_info = self.response_panel.read(cx).last_response_summary();
        let is_loading = self.response_panel.read(cx).is_loading();

        let sep = || div()
            .w(px(1.0))
            .h(px(10.0))
            .bg(theme.colors.border)
            .mx(px(6.0));

        div()
            .id("status-bar")
            .h(px(22.0))
            .w_full()
            .flex()
            .items_center()
            .flex_shrink_0()
            .px(px(10.0))
            .gap(px(0.0))
            .bg(theme.colors.bg_primary)
            .border_t_1()
            .border_color(theme.colors.border)
            // Active env dot + label
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(5.0))
                    .child(
                        div()
                            .size(px(6.0))
                            .bg(theme.colors.accent)
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(theme.colors.text_secondary)
                            .child("Local Dev")
                    )
            )
            .child(sep())
            // Protocol badge
            .child(
                div()
                    .text_size(px(10.0))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(protocol_color)
                    .child(protocol)
            )
            .child(sep())
            // Response info or ready state
            .child(if is_loading {
                div()
                    .flex()
                    .items_center()
                    .gap(px(5.0))
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(theme.colors.text_muted)
                            .child("Sending…")
                    )
                    .into_any_element()
            } else if let Some((status, _, time_ms, size_bytes)) = response_info {
                let status_color = theme.status_color(status);
                let size_str = if size_bytes >= 1024 * 1024 {
                    format!("{:.1} MB", size_bytes as f64 / (1024.0 * 1024.0))
                } else if size_bytes >= 1024 {
                    format!("{:.1} KB", size_bytes as f64 / 1024.0)
                } else {
                    format!("{} B", size_bytes)
                };
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .child(
                        div()
                            .text_size(px(10.0))
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(status_color)
                            .child(format!("{}", status))
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(theme.colors.text_muted)
                            .child("·")
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(theme.colors.text_secondary)
                            .child(format!("{}ms", time_ms))
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(theme.colors.text_muted)
                            .child("·")
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(theme.colors.text_secondary)
                            .child(size_str)
                    )
                    .into_any_element()
            } else {
                div()
                    .text_size(px(10.0))
                    .text_color(theme.colors.text_muted)
                    .child("Ready")
                    .into_any_element()
            })
    }
}
