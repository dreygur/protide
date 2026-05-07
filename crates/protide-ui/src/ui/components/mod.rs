//! Shared UI components

pub mod button;
pub mod code_editor;
pub mod icons;
pub mod modal;
mod text_input;
mod ui_helpers;

pub use ui_helpers::{icon_btn, toolbar_btn};

#[allow(unused_imports)]
pub use text_input::{
    TextInput, TextInputStyle,
    // Standalone helpers for inline text rendering
    render_text_view, render_text_view_with_max, render_text_view_with_max_scrolled,
    index_for_x, effective_click_count,
    // Word boundary helpers
    is_word_char, find_word_start, find_word_end,
};
