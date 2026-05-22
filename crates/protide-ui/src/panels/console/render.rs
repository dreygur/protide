use super::*;
use super::rows;

use gpui::{div, prelude::*, px, ClipboardItem, FontWeight, KeyDownEvent, MouseButton, Render, SharedString, Window};

impl Render for ConsolePanel {
    fn render(&mut self, _window: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let theme = crate::theme::current(cx);
        let entry_count = self.entries.len();
        let entries: Vec<ConsoleEntry> = self.entries.iter().cloned().collect();
        let context_menu = self.context_menu;
        let url_sel = self.url_sel_entry;
        let show_team = self.show_team;
        let show_system = self.show_system;

        // for-loop (not .map closure) so cx reborrow ends each iteration — no capture conflict.
        let mut entry_rows = Vec::new();
        for (i, e) in entries.iter().enumerate() {
            if (show_team   || e.source != ConsoleEntrySource::Team) &&
               (show_system || e.source != ConsoleEntrySource::System)
            {
                entry_rows.push(rows::render_entry_row(i, e, &theme, url_sel, cx));
            }
        }

        div()
            .id("console-panel")
            .w_full().h_full().flex().flex_col()
            .bg(theme.colors.bg_primary)
            .border_t_1().border_color(theme.colors.border)
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                if event.keystroke.modifiers.control && event.keystroke.key.as_str() == "c" {
                    if let Some(idx) = this.url_sel_entry
                        && let Some(entry) = this.entries.get(idx) {
                            cx.write_to_clipboard(ClipboardItem::new_string(entry.url.clone()));
                        }
                    cx.stop_propagation();
                }
            }))
            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                let changed = this.url_sel_entry.is_some() || this.context_menu.is_some();
                this.url_sel_entry = None;
                this.context_menu = None;
                if changed { cx.notify(); }
            }))
            .child(render_header(self, entry_count, &theme, cx))
            .child(
                div()
                    .id("console-entries")
                    .w_full().flex_1().overflow_scroll()
                    .track_scroll(&self.scroll)
                    .when(entries.is_empty(), |el| {
                        el.child(
                            div().w_full().h_full().flex().items_center().justify_center()
                                .child(div().text_size(px(12.0)).text_color(theme.colors.text_muted).child("No requests yet."))
                        )
                    })
                    .children(entry_rows)
            )
            .when_some(context_menu, |el, (idx, pos)| {
                if let Some(entry) = self.entries.get(idx) {
                    el.child(render_context_menu(entry.clone(), pos, &theme, cx))
                } else {
                    el
                }
            })
    }
}

fn render_header(
    panel: &ConsolePanel,
    entry_count: usize,
    theme: &crate::theme::Theme,
    cx: &mut gpui::Context<ConsolePanel>,
) -> impl IntoElement {
    use crate::components::icons::{icon, ICON_CLOSE, ICON_SM};
    div()
        .w_full().h(px(32.0)).flex().items_center().px(px(12.0)).gap(px(8.0))
        .bg(theme.colors.bg_secondary).border_b_1().border_color(theme.colors.border)
        .child(div().text_size(px(11.0)).font_weight(FontWeight::SEMIBOLD).text_color(theme.colors.text_secondary).child("CONSOLE"))
        .child(div().px(px(5.0)).py(px(1.0)).bg(theme.colors.accent.opacity(0.15)).text_size(px(10.0)).text_color(theme.colors.accent)
            .child(SharedString::from(format!("{}", entry_count))))
        .child(div().flex_1())
        .child(level_chip("INFO",  theme.colors.success, theme))
        .child(level_chip("DEBUG", theme.colors.info,    theme))
        .child(level_chip("ERROR", theme.colors.error,   theme))
        .child(
            div().id("console-toggle-team").px(px(5.0)).py(px(1.0)).flex_shrink_0().cursor_pointer()
                .bg(if panel.show_team { theme.colors.team_accent.opacity(0.15) } else { gpui::Hsla::default() })
                .text_size(px(9.0)).font_weight(FontWeight::BOLD)
                .text_color(if panel.show_team { theme.colors.team_accent } else { theme.colors.text_muted.opacity(0.5) })
                .hover(|s| s.bg(theme.colors.team_accent.opacity(0.1)))
                .on_click(cx.listener(|this, _, _, cx| this.toggle_team(cx)))
                .child("TEAM")
        )
        .child(
            div().id("console-toggle-system").px(px(5.0)).py(px(1.0)).flex_shrink_0().cursor_pointer()
                .bg(if panel.show_system { theme.colors.info.opacity(0.15) } else { gpui::Hsla::default() })
                .text_size(px(9.0)).font_weight(FontWeight::BOLD)
                .text_color(if panel.show_system { theme.colors.info } else { theme.colors.text_muted.opacity(0.5) })
                .hover(|s| s.bg(theme.colors.info.opacity(0.1)))
                .on_click(cx.listener(|this, _, _, cx| this.toggle_system(cx)))
                .child("SYS")
        )
        .child(div().w(px(6.0)))
        .child(
            div().id("console-clear").px(px(8.0)).h(px(22.0)).flex().items_center().gap(px(4.0))
                .text_size(px(11.0)).text_color(theme.colors.text_muted).cursor_pointer()
                .hover(|s| s.bg(theme.colors.bg_tertiary))
                .on_click(cx.listener(|this, _, _, cx| this.clear(cx)))
                .child(icon(ICON_CLOSE, ICON_SM, theme.colors.text_muted))
                .child("Clear")
        )
}

fn render_context_menu(
    entry: ConsoleEntry,
    pos: gpui::Point<gpui::Pixels>,
    theme: &crate::theme::Theme,
    cx: &mut gpui::Context<ConsolePanel>,
) -> impl IntoElement {
    use crate::components::icons::{icon, ICON_COPY, ICON_ELLIPSIS, ICON_SM};
    let x = f32::from(pos.x);
    let y = f32::from(pos.y);
    let has_hint     = entry.troubleshoot_hint.is_some();
    let has_error    = entry.error.is_some();
    let hint_text    = entry.troubleshoot_hint.clone().unwrap_or_default();
    let error_detail = entry.error_details();

    gpui::deferred(
        div()
            .absolute().left(px(x)).top(px(y)).w(px(210.0))
            .bg(theme.colors.bg_secondary).border_1().border_color(theme.colors.border)
            .shadow_lg().overflow_hidden()
            .on_mouse_down(MouseButton::Left, cx.listener(|_, _, _, cx| cx.stop_propagation()))
            .child(
                div().flex().flex_col()
                    .child({
                        let curl = entry.as_curl();
                        let bg = theme.colors.bg_tertiary; let tm = theme.colors.text_muted; let tp = theme.colors.text_primary;
                        div().id("console-ctx-curl").w_full().h(px(28.0)).flex().items_center().px(px(12.0)).gap(px(8.0))
                            .cursor_pointer().hover(move |s| s.bg(bg))
                            .on_click(cx.listener(move |this, _, _, cx| { cx.write_to_clipboard(ClipboardItem::new_string(curl.clone())); this.context_menu = None; cx.notify(); }))
                            .child(icon(ICON_COPY, ICON_SM, tm))
                            .child(div().text_size(px(12.0)).text_color(tp).child("Copy as cURL"))
                    })
                    .child({
                        let body = entry.response_body.clone();
                        let bg = theme.colors.bg_tertiary; let tm = theme.colors.text_muted; let tp = theme.colors.text_primary;
                        div().id("console-ctx-body").w_full().h(px(28.0)).flex().items_center().px(px(12.0)).gap(px(8.0))
                            .cursor_pointer().hover(move |s| s.bg(bg))
                            .on_click(cx.listener(move |this, _, _, cx| { cx.write_to_clipboard(ClipboardItem::new_string(body.clone())); this.context_menu = None; cx.notify(); }))
                            .child(icon(ICON_ELLIPSIS, ICON_SM, tm))
                            .child(div().text_size(px(12.0)).text_color(tp).child("Copy Response"))
                    })
                    .when(has_error, |el| {
                        let detail = error_detail.clone();
                        let bg = theme.colors.bg_tertiary; let tm = theme.colors.text_muted; let tp = theme.colors.text_primary;
                        el.child(
                            div().id("console-ctx-err-detail").w_full().h(px(28.0)).flex().items_center().px(px(12.0)).gap(px(8.0))
                                .cursor_pointer().hover(move |s| s.bg(bg))
                                .on_click(cx.listener(move |this, _, _, cx| { cx.write_to_clipboard(ClipboardItem::new_string(detail.clone())); this.context_menu = None; cx.notify(); }))
                                .child(icon(ICON_COPY, ICON_SM, tm))
                                .child(div().text_size(px(12.0)).text_color(tp).child("Copy Error Details"))
                        )
                    })
                    .when(has_hint, |el| el.child(div().h(px(1.0)).mx(px(8.0)).my(px(2.0)).bg(theme.colors.border)))
                    .when(has_hint, |el| el.child(
                        div().id("console-ctx-troubleshoot").w_full().px(px(12.0)).py(px(6.0))
                            .flex().flex_col().gap(px(2.0)).cursor_pointer()
                            .hover(|s| s.bg(theme.colors.bg_tertiary))
                            .on_click(cx.listener(move |this, _, _, cx| { cx.write_to_clipboard(ClipboardItem::new_string(hint_text.clone())); this.context_menu = None; cx.notify(); }))
                            .child(
                                div().flex().items_center().gap(px(6.0))
                                    .child(div().size(px(12.0)).flex().items_center().justify_center().bg(theme.colors.warning.opacity(0.2)).text_size(px(8.0)).font_weight(FontWeight::BOLD).text_color(theme.colors.warning).child("?"))
                                    .child(div().text_size(px(12.0)).font_weight(FontWeight::SEMIBOLD).text_color(theme.colors.warning).child("Troubleshoot"))
                            )
                            .child(div().text_size(px(10.0)).text_color(theme.colors.text_muted).child("Click to copy steps to clipboard"))
                    ))
            )
    ).with_priority(10)
}

fn level_chip(label: &'static str, color: gpui::Hsla, _theme: &crate::theme::Theme) -> gpui::AnyElement {
    div().px(px(5.0)).py(px(1.0)).bg(color.opacity(0.12)).text_size(px(9.0))
        .font_weight(FontWeight::BOLD).text_color(color.opacity(0.7)).child(label)
        .into_any_element()
}
