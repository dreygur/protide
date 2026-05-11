use crate::theme;
use crate::ui::components::icons::{ICON_MD, ICON_SM, icon};
use gpui::prelude::*;
use gpui::{AppContext, Div, ElementId, Stateful, div, px};

/// 22px icon-only button with standard hover. Returns Stateful<Div> — caller adds .on_click().
pub fn icon_btn(id: impl Into<ElementId>, icon_src: &'static str, cx: &impl AppContext) -> Stateful<Div> {
    let theme = theme::current(cx);
    div()
        .id(id.into())
        .size(px(22.0))
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .hover(|s| {
            s.bg(theme.colors.bg_tertiary)
                .text_color(theme.colors.text_primary)
        })
        .child(icon(icon_src, ICON_MD, theme.colors.text_muted))
}

/// 18×18px ghost icon button for use inside ActionRow actions.
/// Invisible background; reveals a subtle bg on hover.  Caller adds .on_click().
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
        .child(icon(icon_src, ICON_SM, theme.colors.text_muted))
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
