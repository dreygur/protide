//! Shared UI components

pub mod action_row;
pub mod code_editor;
pub mod icons;
pub mod modal;
pub mod selectable_text;
pub mod text_view;
mod ui_helpers;

pub use action_row::ActionRow;
pub use ui_helpers::{ghost_action_btn, icon_btn, toolbar_btn, tooltip_text};

pub use text_view::{
    render_text_view_with_max, render_text_view_with_max_scrolled,
    find_word_start, find_word_end,
};
