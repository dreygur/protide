use gpui::{Context, FontWeight, IntoElement, MouseButton, ParentElement, Styled, div, hsla, px, prelude::*};
use gpui_component::Sizable;
use super::*;

impl MainWindow {
    pub(super) fn render_pairing_flyout_panel(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let status = self.presence.connection_status.clone();
        let tick = self.presence.handshake_tick;

        let neon = hsla(120.0 / 360.0, 1.0, 0.45, 1.0);

        let (connect_bg, connect_border, connect_text_color, connect_label, interactive) =
            match (&status, tick) {
                (ConnectionStatus::Handshaking, true) => {
                    (neon.opacity(0.15), neon, neon, "Connecting…", false)
                }
                (ConnectionStatus::Handshaking, false) => {
                    (neon.opacity(0.15), neon.opacity(0.25), neon, "Connecting…", false)
                }
                _ => (
                    theme.colors.team_accent.opacity(0.12),
                    theme.colors.team_accent,
                    theme.colors.team_accent,
                    "Connect",
                    true,
                ),
            };

        let error_msg: Option<String> = match &status {
            ConnectionStatus::Error(msg) => Some(msg.clone()),
            _ => None,
        };

        let connect_btn_base = div()
            .id("join-connect-btn")
            .flex_1()
            .h(px(26.0))
            .flex()
            .items_center()
            .justify_center()
            .bg(connect_bg)
            .border_1()
            .border_color(connect_border)
            .child(
                div()
                    .text_size(px(10.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(connect_text_color)
                    .child(connect_label),
            );

        let connect_btn: gpui::AnyElement = if interactive {
            connect_btn_base
                .cursor_pointer()
                .hover(|s| s.opacity(0.85))
                .on_click(cx.listener(|this, _, _, cx| this.connect_peer(cx)))
                .into_any_element()
        } else {
            connect_btn_base.into_any_element()
        };

        let paste_btn_base = div()
            .id("join-paste-btn")
            .flex_1()
            .h(px(26.0))
            .flex()
            .items_center()
            .justify_center()
            .bg(theme.colors.bg_tertiary)
            .border_1()
            .border_color(theme.colors.border)
            .child(
                div()
                    .text_size(px(10.0))
                    .text_color(if interactive {
                        theme.colors.text_secondary
                    } else {
                        theme.colors.text_muted
                    })
                    .child("Paste & Join"),
            );

        let paste_btn: gpui::AnyElement = if interactive {
            paste_btn_base
                .cursor_pointer()
                .hover(|s| s.bg(theme.colors.bg_elevated))
                .on_click(cx.listener(|this, _, window, cx| this.paste_and_join(window, cx)))
                .into_any_element()
        } else {
            paste_btn_base.into_any_element()
        };

        let join_section = div()
            .flex()
            .flex_col()
            .gap(px(6.0))
            .child(Input::new(&self.join_input).with_size(gpui_component::Size::Small))
            .child(div().flex().gap(px(6.0)).child(paste_btn).child(connect_btn))
            .when_some(error_msg, |el, msg| {
                let error_color = theme.colors.error;
                el.child(
                    div()
                        .w_full()
                        .px(px(8.0))
                        .py(px(5.0))
                        .bg(error_color.opacity(0.08))
                        .border_1()
                        .border_color(error_color.opacity(0.35))
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .child(
                            div()
                                .flex_1()
                                .text_size(px(10.0))
                                .text_color(error_color)
                                .child(msg),
                        )
                        .child(
                            div()
                                .id("handshake-retry-btn")
                                .px(px(6.0))
                                .py(px(2.0))
                                .text_size(px(9.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(error_color)
                                .cursor_pointer()
                                .hover(move |s| s.bg(error_color.opacity(0.15)))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.presence.reset_connection();
                                    this.handshake_started = None;
                                    cx.notify();
                                }))
                                .child("Retry"),
                        ),
                )
            })
            .into_any_element();

        self.presence.render_pairing_flyout(&theme, join_section)
    }

    pub(super) fn render_menu_dropdown(&self, cx: &Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let toolbar_h = 40.0f32;

        type ActionFn = Box<dyn Fn(&mut gpui::Window, &mut gpui::App)>;
        let items: Vec<(&str, &str, ActionFn)> = match self.open_menu {
            Some(0) => vec![
                ("About Protide",     "",          Box::new(|w, cx| w.dispatch_action(Box::new(ShowAbout), cx))),
                ("---",               "",          Box::new(|_, _| {})),
                ("Quit",              "Ctrl+Q",    Box::new(|w, cx| w.dispatch_action(Box::new(Quit), cx))),
            ],
            Some(1) => vec![
                ("Send Request",      "Ctrl+Enter", Box::new(|w, cx| w.dispatch_action(Box::new(SendRequest), cx))),
                ("Save Request",      "Ctrl+S",     Box::new(|w, cx| w.dispatch_action(Box::new(SaveRequest), cx))),
            ],
            Some(2) => vec![
                ("Toggle Sidebar",      "Ctrl+B",       Box::new(|w, cx| w.dispatch_action(Box::new(ToggleSidebar), cx))),
                ("Toggle Mock Server",  "Ctrl+Shift+M", Box::new(|w, cx| w.dispatch_action(Box::new(ToggleMockServer), cx))),
                ("Toggle API Explorer", "Ctrl+Shift+D", Box::new(|w, cx| w.dispatch_action(Box::new(ToggleDocs), cx))),
            ],
            Some(3) => vec![
                ("Keyboard Shortcuts","F1",          Box::new(|w, cx| w.dispatch_action(Box::new(ShowHelp), cx))),
            ],
            _ => vec![],
        };

        let left_px = match self.open_menu {
            Some(0) => 88.0,
            Some(1) => 148.0,
            Some(2) => 220.0,
            Some(3) => 272.0,
            _       => 88.0,
        };

        div()
            .id("menu-dropdown")
            .absolute()
            .top(px(toolbar_h))
            .left(px(left_px))
            .min_w(px(200.0))
            .py(px(4.0))
            .bg(theme.colors.bg_elevated)
            .border_1()
            .border_color(theme.colors.border)
            .shadow_lg()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .children(items.into_iter().enumerate().map(|(i, (label, hint, action))| {
                if label == "---" {
                    return div()
                        .id(("menu-sep", i))
                        .my(px(3.0))
                        .mx(px(6.0))
                        .h(px(1.0))
                        .bg(theme.colors.border)
                        .into_any_element();
                }
                div()
                    .id(("menu-item", i))
                    .px(px(12.0)).py(px(7.0))
                    .flex().items_center().justify_between()
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.colors.bg_tertiary))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.open_menu = None;
                        cx.notify();
                        action(window, cx);
                    }))
                    .child(
                        div().text_size(px(12.0)).text_color(theme.colors.text_primary).child(label)
                    )
                    .when(!hint.is_empty(), |el| el.child(
                        div().text_size(px(10.0)).text_color(theme.colors.text_muted).ml(px(24.0)).child(hint)
                    ))
                    .into_any_element()
            }))
    }
}
