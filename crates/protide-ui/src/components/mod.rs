//! Shared UI components

pub mod action_row;
pub mod code_editor;
pub mod icons;
pub mod modal;
pub mod selectable_text;
mod text_input;
mod ui_helpers;

pub use action_row::ActionRow;
pub use ui_helpers::{ghost_action_btn, icon_btn, toolbar_btn, tooltip_text};

#[allow(unused_imports)]
pub use text_input::{
    TextInput, TextInputStyle,
    // Standalone helpers for inline text rendering
    render_text_view_with_max, render_text_view_with_max_scrolled,
    index_for_x, effective_click_count,
    // Word boundary helpers
    is_word_char, find_word_start, find_word_end,
};
