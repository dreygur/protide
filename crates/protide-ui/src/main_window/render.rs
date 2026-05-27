use gpui::{App, Context, IntoElement, MouseButton, ParentElement, Render, Styled, Window, deferred, div, px, prelude::*};
use gpui_component::WindowExt;
use super::*;

impl Render for MainWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some((title, msg, is_confirm)) = self.pending_alert.take() {
            if is_confirm {
                if let Some(path) = self.pending_delete.take() {
                    let self_weak = cx.entity().downgrade();
                    window.open_alert_dialog(cx, move |alert, _, _| {
                        let sw = self_weak.clone();
                        let p = path.clone();
                        alert
                            .confirm()
                            .title(title.clone())
                            .description(msg.clone())
                            .on_ok(move |_, _, cx| {
                                sw.update(cx, |win, cx| {
                                    win.explorer.update(cx, |panel, cx| panel.execute_delete(p.clone(), cx));
                                })
                                .ok();
                                true
                            })
                    });
                }
            } else {
                window.open_alert_dialog(cx, move |alert, _, _| {
                    alert.title(title.clone()).description(msg.clone())
                });
            }
        }
        let theme = theme::current(cx);
        let show_response = self.request_panel.read(cx).shows_response_panel();
        let show_codegen = self.request_panel.read(cx).codegen_content.is_some();
        let import_modal: Option<gpui::AnyElement> = if self.request_panel.read(cx).import_modal_open {
            Some(self.request_panel.update(cx, |p, cx| p.render_import_modal(cx)))
        } else {
            None
        };
        let is_dragging = self.drag_sidebar.is_some()
            || self.drag_response.is_some()
            || self.drag_mock_server.is_some()
            || self.drag_codegen.is_some()
            || self.drag_console.is_some()
            || self.drag_docs.is_some();
        let is_col_drag = self.drag_sidebar.is_some()
            || self.drag_mock_server.is_some()
            || self.drag_codegen.is_some()
            || self.drag_docs.is_some();

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.colors.bg_primary)
            .border_1()
            .border_color(theme.colors.border)
            .text_color(theme.colors.text_primary)
            .track_focus(&self.focus)
            .key_context("MainWindow")
            .on_action(cx.listener(|this, _: &SendRequest, _, cx| {
                this.request_panel.update(cx, |p, cx| p.send_request(cx));
            }))
            .on_action(cx.listener(|this, _: &SaveRequest, _, cx| {
                this.request_panel.update(cx, |p, cx| p.save_request(cx));
            }))
            .on_action(cx.listener(|this, _: &ToggleSidebar, _, cx| this.toggle_sidebar(cx)))
            .on_action(cx.listener(|this, _: &ToggleMockServer, _, cx| this.toggle_mock_server(cx)))
            .on_action(cx.listener(|this, _: &ToggleConsole, _, cx| this.toggle_console(cx)))
            .on_action(cx.listener(|this, _: &ToggleDocs, _, cx| this.toggle_docs(cx)))
            .on_action(cx.listener(|this, _: &ShowHelp, _, cx| { this.show_help = true; cx.notify(); }))
            .on_action(cx.listener(|this, _: &ShowAbout, _, cx| { this.show_about = true; cx.notify(); }))
            .on_action(cx.listener(|this, _: &DismissOverlay, _, cx| this.dismiss_overlay(cx)))
            .on_action(|_: &Quit, _, cx: &mut App| { cx.quit(); })
            .child(self.render_title_bar(cx))
            .child(
                div()
                    .flex_1()
                    .flex()
                    .overflow_hidden()
                    .when(self.sidebar_collapsed, |el| el.child(self.render_collapsed_sidebar(cx)))
                    .when(!self.sidebar_collapsed, |el| el.child(self.render_sidebar(cx)))
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .overflow_hidden()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .w_full()
                                    .overflow_hidden()
                                    .when(show_response, |el| el.flex_shrink_0().min_h(px(150.0)).h(px(self.request_height)))
                                    .when(!show_response, |el| el.flex_1())
                                    .child(self.request_panel.clone()),
                            )
                            .when(show_response, |el| {
                                let collapsed = self.response_collapsed;
                                el
                                    // 4-px resize handle — only while expanded
                                    .when(!collapsed, |el| el.child(
                                        div()
                                            .id("response-resize-handle")
                                            .w_full()
                                            .h(px(4.0))
                                            .flex_shrink_0()
                                            .border_t_1()
                                            .border_color(theme.colors.border)
                                            .cursor_row_resize()
                                            .hover(|s| s.bg(theme.colors.accent.opacity(0.25)))
                                            .on_mouse_down(
                                                MouseButton::Left,
                                                cx.listener(|this, event: &gpui::MouseDownEvent, _, _| {
                                                    this.drag_response = Some((f32::from(event.position.y), this.request_height));
                                                }),
                                            ),
                                    ))
                                    // Accordion strip: chevron + "Response" + live status
                                    .child(self.render_response_strip(collapsed, cx))
                                    // Response panel body
                                    .when(!collapsed, |el| el.child(
                                        div()
                                            .flex_1()
                                            .min_h(px(100.0))
                                            .w_full()
                                            .overflow_hidden()
                                            .child(self.response_panel.clone()),
                                    ))
                            })
                            .when(self.show_console, |el| {
                                el.child(
                                    div()
                                        .id("console-resize-handle")
                                        .w_full()
                                        .h(px(4.0))
                                        .flex_shrink_0()
                                        .border_t_1()
                                        .border_color(theme.colors.border)
                                        .cursor_row_resize()
                                        .hover(|s| s.bg(theme.colors.accent.opacity(0.25)))
                                        .on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(|this, event: &gpui::MouseDownEvent, _, _| {
                                                this.drag_console = Some((f32::from(event.position.y), this.console_height));
                                            }),
                                        ),
                                )
                                .child(
                                    div()
                                        .w_full()
                                        .h(px(self.console_height))
                                        .flex_shrink_0()
                                        .overflow_hidden()
                                        .child(self.console_panel.clone()),
                                )
                            }),
                    )
                    .when(self.show_mock_server, |el| {
                        el.child(
                            div()
                                .id("mock-server-resize-handle")
                                .w(px(4.0))
                                .h_full()
                                .flex_shrink_0()
                                .border_r_1()
                                .border_color(theme.colors.border)
                                .cursor_col_resize()
                                .hover(|s| s.bg(theme.colors.accent.opacity(0.25)))
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(|this, event: &gpui::MouseDownEvent, _, _| {
                                        this.drag_mock_server = Some((f32::from(event.position.x), this.mock_server_width));
                                    }),
                                ),
                        )
                        .child(
                            div()
                                .w(px(self.mock_server_width))
                                .h_full()
                                .flex_shrink_0()
                                .overflow_hidden()
                                .child(self.mock_server_panel.clone()),
                        )
                    })
                    .when(self.show_docs, |el| {
                        el.child(
                            div()
                                .id("docs-resize-handle")
                                .w(px(4.0))
                                .h_full()
                                .flex_shrink_0()
                                .border_l_1()
                                .border_color(theme.colors.border)
                                .cursor_col_resize()
                                .hover(|s| s.bg(theme.colors.accent.opacity(0.25)))
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(|this, event: &gpui::MouseDownEvent, _, _| {
                                        this.drag_docs = Some((f32::from(event.position.x), this.docs_width));
                                    }),
                                ),
                        )
                        .child(
                            div()
                                .w(px(self.docs_width))
                                .h_full()
                                .flex_shrink_0()
                                .overflow_hidden()
                                .child(self.docs_panel.clone()),
                        )
                    })
                    .when(show_codegen, |el| {
                        el.child(
                            div()
                                .id("codegen-resize-handle")
                                .w(px(4.0))
                                .h_full()
                                .flex_shrink_0()
                                .border_l_1()
                                .border_color(theme.colors.border)
                                .cursor_col_resize()
                                .hover(|s| s.bg(theme.colors.accent.opacity(0.25)))
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(|this, event: &gpui::MouseDownEvent, _, _| {
                                        this.drag_codegen = Some((f32::from(event.position.x), this.codegen_panel_width));
                                    }),
                                ),
                        )
                        .child(self.render_codegen_panel(cx))
                    })
                    .when(is_dragging, |el| el.child(self.render_drag_overlay(is_col_drag, cx))),
            )
            .child(self.render_status_bar(cx))
            .when(self.presence.show_pairing, |el| {
                let flyout = self.render_pairing_flyout_panel(cx);
                let toolbar_h = theme.sizes.toolbar;
                el
                    .child(
                        div()
                            .absolute().top(toolbar_h).left_0().w_full().h_full()
                            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                                this.presence.show_pairing = false;
                                this.presence.reset_connection();
                                cx.notify();
                            }))
                    )
                    .child(
                        deferred(
                            div()
                                .absolute()
                                .top(px(40.0))
                                .left(px(300.0))
                                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                                .child(flyout)
                        ).with_priority(10)
                    )
            })
            .when(self.open_menu.is_some(), |el| el
                .child(
                    div()
                        .absolute().top_0().left_0().w_full().h_full()
                        .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                            this.open_menu = None;
                            cx.notify();
                        }))
                )
                .child(deferred(self.render_menu_dropdown(cx)).with_priority(10))
            )
            .when_some(import_modal, |el, modal| el.child(modal))
            .when(self.show_help, |el| el.child(self.render_help_overlay(cx)))
            .when(self.show_about, |el| el.child(self.render_about_overlay(cx)))
            .when(self.show_runner, |el| {
                let theme = theme::current(cx);
                el.child(
                    div()
                        .absolute().top_0().left_0().w_full().h_full()
                        .flex().items_center().justify_center()
                        .bg(theme.colors.bg_primary.opacity(0.6))
                        .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                            this.close_runner(cx);
                        }))
                        .child(
                            div()
                                .w(px(440.0)).h(px(520.0))
                                .rounded_md()
                                .overflow_hidden()
                                .shadow_lg()
                                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                                .child(self.runner_panel.clone()),
                        )
                )
            })
    }
}
