use crate::theme;
use crate::components::icons::{ICON_MD, ICON_SM, icon};
use gpui::prelude::*;
use gpui::{App, AppContext, Div, ElementId, SharedString, Stateful, Window, div, px, white};

/// 22px icon-only button. Icon is dim white normally, full white on hover.
/// Caller adds .on_click() and optionally .tooltip().
pub fn icon_btn(id: impl Into<ElementId>, icon_src: &'static str, cx: &impl AppContext) -> Stateful<Div> {
    let theme = theme::current(cx);
    div()
        .id(id.into())
        .size(px(22.0))
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .hover(|s| s.bg(theme.colors.bg_tertiary))
        .child(
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .opacity(0.45)
                .hover(|s| s.opacity(1.0))
                .child(icon(icon_src, ICON_MD, white()))
        )
}

/// 18×18px ghost icon button for use inside ActionRow actions.
/// Caller adds .on_click().
pub fn ghost_action_btn(
    id: impl Into<ElementId>,
    icon_src: &'static str,
    cx: &impl AppContext,
) -> Stateful<Div> {
    let theme = theme::current(cx);
    div()
        .id(id.into())
        .size(px(18.0))
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .hover(|s| s.bg(theme.colors.bg_elevated))
        .child(
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .opacity(0.45)
                .hover(|s| s.opacity(1.0))
                .child(icon(icon_src, ICON_SM, white()))
        )
}

/// Compact labeled toolbar button (border, 11px text, subtle hover).
/// Caller adds .child() for icon/label and .on_click().
pub fn toolbar_btn(id: impl Into<ElementId>, cx: &impl AppContext) -> Stateful<Div> {
    let theme = theme::current(cx);
    div()
        .id(id.into())
        .h(px(32.0))
        .px(px(10.0))
        .flex()
        .items_center()
        .justify_center()
        .gap(px(4.0))
        .text_size(px(11.0))
        .text_color(theme.colors.text_secondary)
        .border_1()
        .border_color(theme.colors.border)
        .cursor_pointer()
        .hover(|s| {
            s.bg(theme.colors.bg_tertiary)
                .border_color(theme.colors.text_muted)
        })
}

/// Simple text tooltip view.
struct TooltipText(SharedString);

impl gpui::Render for TooltipText {
    fn render(&mut self, _window: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        div()
            .px(px(8.0))
            .py(px(3.0))
            .bg(theme.colors.bg_elevated)
            .border_1()
            .border_color(theme.colors.border)
            .text_size(px(11.0))
            .text_color(theme.colors.text_primary)
            .child(self.0.clone())
    }
}

/// Returns a tooltip builder closure for use with `.tooltip(tooltip_text("..."))`.
pub fn tooltip_text(text: impl Into<SharedString>) -> impl Fn(&mut Window, &mut App) -> gpui::AnyView {
    let text: SharedString = text.into();
    move |_window, cx| cx.new(|_| TooltipText(text.clone())).into()
}
