//! Shared UI components

pub mod button;
pub mod code_editor;
mod text_input;

pub use button::{Button, ButtonVariant, ButtonSize, ButtonStyles};

#[allow(unused_imports)]
pub use text_input::{
    TextInput, TextInputStyle,
    // Standalone helpers for inline text rendering
    render_text_view, render_text_view_with_max, index_for_x, effective_click_count,
    // Word boundary helpers
    is_word_char, find_word_start, find_word_end,
};
