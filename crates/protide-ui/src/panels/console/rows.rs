use super::*;

use gpui::{div, prelude::*, px, ClipboardItem, MouseButton, SharedString};

pub(super) fn render_entry_row(
    i: usize,
    entry: &ConsoleEntry,
    theme: &crate::theme::Theme,
    url_sel: Option<usize>,
    cx: &mut gpui::Context<ConsolePanel>,
) -> impl gpui::IntoElement + use<> {
    let status = entry.status;
    let is_ws  = entry.protocol == "WebSocket" || entry.protocol == "Socket.IO";
    let is_team = entry.source == ConsoleEntrySource::Team;

    let row_color = if is_team {
        theme.colors.team_accent
    } else {
        match entry.level {
            LogLevel::Error => theme.colors.error,
            LogLevel::Debug => theme.colors.info,
            LogLevel::Info  => {
                if entry.error.is_some() || status >= 500 {
                    theme.colors.error
                } else if status >= 400 {
                    theme.colors.method_delete
                } else if status >= 300 {
                    theme.colors.method_patch
                } else if status == 0 {
                    theme.colors.text_muted
                } else {
                    theme.colors.status_success
                }
            }
        }
    };

    let level_bar = if is_team {
        theme.colors.team_accent
    } else {
        match entry.level {
            LogLevel::Error => theme.colors.error,
            LogLevel::Debug => theme.colors.info,
            LogLevel::Info  => theme.colors.success.opacity(0.0),
        }
    };

    let method_color = theme.method_color(&entry.method);
    let has_hint = entry.troubleshoot_hint.is_some();
    let has_error = entry.error.is_some();

    let base_row = div()
        .id(SharedString::from(format!("console-entry-{}", i)))
        .w_full()
        .h(px(24.0))
        .flex()
        .items_center()
        .gap(px(10.0))
        .border_b_1()
        .border_color(theme.colors.border.opacity(0.4))
        .hover(|s| s.bg(theme.colors.hover_overlay))
        .on_mouse_down(
            MouseButton::Right,
            cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
                this.context_menu = Some((i, event.position));
                cx.stop_propagation();
                cx.notify();
            }),
        )
        .child(
            div()
                .w(px(3.0))
                .h(px(24.0))
                .flex_shrink_0()
                .bg(level_bar)
        )
        .child(
            div()
                .w(px(56.0))
                .flex_shrink_0()
                .text_size(px(10.0))
                .text_color(theme.colors.text_muted)
                .child(SharedString::from(
                    entry.timestamp.format("%H:%M:%S").to_string()
                ))
        );

    if is_team {
        base_row
            .child(
                div()
                    .px(px(4.0))
                    .py(px(1.0))
                    .flex_shrink_0()
                    .bg(theme.colors.team_accent.opacity(0.12))
                    .text_size(px(9.0))
                    .font_weight(gpui::FontWeight::BOLD)
                    .text_color(theme.colors.team_accent)
                    .child("TEAM")
            )
            .child(
                div()
                    .id(SharedString::from(format!("console-url-{}", i)))
                    .flex_1()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .text_size(px(11.5))
                    .text_color(theme.colors.text_primary)
                    .whitespace_nowrap()
                    .cursor_text()
                    .when(url_sel == Some(i), |el| el.bg(theme.colors.accent.opacity(0.15)))
                    .child(SharedString::from(entry.url.clone()))
                    .on_mouse_down(MouseButton::Left,
                        cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
                            cx.stop_propagation();
                            if event.click_count >= 2 {
                                this.url_sel_entry = Some(i);
                                if let Some(e) = this.entries.get(i) {
                                    cx.write_to_clipboard(ClipboardItem::new_string(e.url.clone()));
                                }
                            } else {
                                this.url_sel_entry = if this.url_sel_entry == Some(i) { None } else { Some(i) };
                            }
                            cx.notify();
                        })
                    )
            )
            .child(div().w(px(108.0)).flex_shrink_0())
    } else {
        base_row
            .when(entry.level == LogLevel::Debug, |el| {
                el.child(
                    div()
                        .px(px(4.0))
                        .py(px(1.0))
                        .flex_shrink_0()
                        .bg(theme.colors.info.opacity(0.12))
                        .text_size(px(9.0))
                        .font_weight(gpui::FontWeight::BOLD)
                        .text_color(theme.colors.info)
                        .child("DBG")
                )
            })
            .child(
                div()
                    .px(px(4.0))
                    .py(px(1.0))
                    .min_w(px(36.0))
                    .flex_shrink_0()
                    .flex()
                    .justify_center()
                    .bg(theme.colors.accent.opacity(0.1))
                    .text_size(px(9.0))
                    .font_weight(gpui::FontWeight::BOLD)
                    .text_color(theme.colors.accent)
                    .child(SharedString::from(entry.protocol.clone()))
            )
            .when(!is_ws, |el| {
                el.child(
                    div()
                        .w(px(46.0))
                        .flex_shrink_0()
                        .text_size(px(10.0))
                        .font_weight(gpui::FontWeight::BOLD)
                        .text_color(method_color)
                        .child(SharedString::from(entry.method.clone()))
                )
            })
            .child(
                div()
                    .id(SharedString::from(format!("console-url-{}", i)))
                    .flex_1()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .text_size(px(11.5))
                    .text_color(theme.colors.text_primary)
                    .whitespace_nowrap()
                    .cursor_text()
                    .when(url_sel == Some(i), |el| el.bg(theme.colors.accent.opacity(0.15)))
                    .child(SharedString::from(entry.url.clone()))
                    .on_mouse_down(MouseButton::Left,
                        cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
                            cx.stop_propagation();
                            if event.click_count >= 2 {
                                this.url_sel_entry = Some(i);
                                if let Some(e) = this.entries.get(i) {
                                    cx.write_to_clipboard(ClipboardItem::new_string(e.url.clone()));
                                }
                            } else {
                                this.url_sel_entry = if this.url_sel_entry == Some(i) { None } else { Some(i) };
                            }
                            cx.notify();
                        })
                    )
            )
            .when(has_hint, |el| {
                el.child(
                    div()
                        .w(px(14.0))
                        .h(px(14.0))
                        .flex_shrink_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(theme.colors.warning.opacity(0.2))
                        .text_size(px(9.0))
                        .font_weight(gpui::FontWeight::BOLD)
                        .text_color(theme.colors.warning)
                        .child("?")
                )
            })
            .child(
                div()
                    .w(px(54.0))
                    .flex_shrink_0()
                    .text_size(px(11.0))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(row_color)
                    .child(SharedString::from(if has_error {
                        "ERR".to_string()
                    } else if status == 0 {
                        "-".to_string()
                    } else {
                        status.to_string()
                    }))
            )
            .child(
                div()
                    .w(px(54.0))
                    .flex_shrink_0()
                    .pr(px(8.0))
                    .text_size(px(10.5))
                    .text_color(theme.colors.text_muted)
                    .child(SharedString::from(if entry.duration_ms > 0 {
                        format!("{}ms", entry.duration_ms)
                    } else {
                        String::new()
                    }))
            )
    }
}
