use gpui::{App, Context, FontWeight, IntoElement, MouseButton, ParentElement, Styled, div, px, prelude::*};
use super::*;

impl MainWindow {
    pub(super) fn render_title_bar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let show_mock = self.show_mock_server;

        div()
            .id("titlebar")
            .h(theme.sizes.toolbar)
            .w_full()
            .flex()
            .items_center()
            .bg(theme.colors.bg_primary)
            .border_b_1()
            .border_color(theme.colors.border)
            .child(
                div()
                    .id("titlebar-drag")
                    .flex()
                    .items_center()
                    .gap(px(7.0))
                    .px(px(8.0))
                    .h_full()
                    .cursor_pointer()
                    .on_mouse_down(MouseButton::Left, |_, window, _cx: &mut App| {
                        window.start_window_move();
                    })
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
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(theme.colors.bg_primary)
                                    .child("P"),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .font_weight(FontWeight::BOLD)
                            .text_color(theme.colors.text_primary)
                            .child("Protide"),
                    ),
            )
            .child({
                let open = self.open_menu;
                let menus: &[(u8, &str)] = &[(0, "Protide"), (1, "Request"), (2, "View"), (3, "Help")];
                div()
                    .flex().items_center().h_full()
                    .children(menus.iter().map(|&(id, label)| {
                        let is_open = open == Some(id);
                        div()
                            .id(("menu-btn", id as usize))
                            .h_full().px(px(10.0))
                            .flex().items_center()
                            .cursor_pointer()
                            .text_size(px(12.0))
                            .when(is_open, |el| el
                                .bg(theme.colors.bg_tertiary)
                                .text_color(theme.colors.text_primary)
                            )
                            .when(!is_open, |el| el
                                .text_color(theme.colors.text_secondary)
                                .hover(|s| s.bg(theme.colors.bg_elevated).text_color(theme.colors.text_primary))
                            )
                            .child(label)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.open_menu = if this.open_menu == Some(id) { None } else { Some(id) };
                                cx.notify();
                            }))
                    }))
            })
            .child(
                div()
                    .id("presence-bar")
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.presence.show_pairing = !this.presence.show_pairing;
                        cx.notify();
                    }))
                    .child(self.presence.render_presence_bar(&theme))
            )
            .child(div().flex_1().h_full().on_mouse_down(
                MouseButton::Left,
                |_, window, _cx: &mut App| {
                    window.start_window_move();
                },
            ))
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
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(if show_mock {
                                theme.colors.accent
                            } else {
                                theme.colors.text_secondary
                            })
                            .child("Mock Server"),
                    ),
            )
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
                            .child(icon(ICON_MINIMIZE, 12.0, theme.colors.text_secondary)),
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
                                window.zoom_window();
                            })
                            .child(icon(ICON_MAXIMIZE, 12.0, theme.colors.text_secondary)),
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
                            .hover(|s| s.bg(theme.colors.error).text_color(theme.colors.bg_primary))
                            .on_click(|_, _window, cx: &mut App| {
                                cx.quit();
                            })
                            .child(icon(ICON_WINDOW_CLOSE, 12.0, theme.colors.text_secondary)),
                    ),
            )
    }
}
