//! Themed modal overlay — replaces OS-native rfd::MessageDialog.

use gpui::{div, prelude::*, px, AnyElement, FontWeight, Hsla, IntoElement, ParentElement, Styled};

use crate::theme::Theme;
use crate::ui::components::icons::{icon, ICON_INFO, ICON_CIRCLE_X};

#[derive(Clone, Debug, PartialEq)]
pub enum ModalLevel {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ModalKind {
    Alert,
    Confirm,
}

#[derive(Clone, Debug)]
pub struct ModalState {
    pub title: String,
    pub message: String,
    pub level: ModalLevel,
    pub kind: ModalKind,
}

impl ModalState {
    pub fn info(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self { title: title.into(), message: message.into(), level: ModalLevel::Info, kind: ModalKind::Alert }
    }
    pub fn warning(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self { title: title.into(), message: message.into(), level: ModalLevel::Warning, kind: ModalKind::Alert }
    }
    pub fn error(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self { title: title.into(), message: message.into(), level: ModalLevel::Error, kind: ModalKind::Alert }
    }
    pub fn confirm(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self { title: title.into(), message: message.into(), level: ModalLevel::Info, kind: ModalKind::Confirm }
    }
}

/// Renders a full-panel backdrop + themed card.
/// `buttons` is the caller-built button row (use `cx.listener` for handlers).
pub fn render_modal_shell(state: &ModalState, theme: &Theme, buttons: AnyElement) -> impl IntoElement {
    let level_color = match state.level {
        ModalLevel::Info => theme.colors.info,
        ModalLevel::Warning => theme.colors.warning,
        ModalLevel::Error => theme.colors.error,
    };
    let level_icon = match state.level {
        ModalLevel::Error => ICON_CIRCLE_X,
        _ => ICON_INFO,
    };
    let title = state.title.clone();
    let message = state.message.clone();

    div()
        .absolute()
        .top_0()
        .left_0()
        .w_full()
        .h_full()
        .flex()
        .items_center()
        .justify_center()
        .p(px(20.0))
        .bg(Hsla { h: 0.0, s: 0.0, l: 0.0, a: 0.6 })
        .child(
            div()
                .w(px(480.0))
                .max_w_full()
                .bg(theme.colors.bg_elevated)
                .border_1()
                .border_color(theme.colors.border)
                .shadow_lg()
                .flex()
                .flex_col()
                .p(px(20.0))
                .gap(px(14.0))
                // Title row
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .child(icon(level_icon, 15.0, level_color))
                        .child(
                            div()
                                .text_size(px(13.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(theme.colors.text_primary)
                                .child(title)
                        )
                )
                // Message
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(theme.colors.text_secondary)
                        .child(message)
                )
                // Buttons
                .child(buttons)
        )
}
