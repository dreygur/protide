//! Global key bindings and the labels the UI shows for them.
//!
//! `secondary` is GPUI's platform modifier: cmd on macOS, ctrl elsewhere.

use super::{
    DismissOverlay, Quit, SaveRequest, SendRequest, ShowAbout, ShowHelp, ToggleConsole, ToggleDocs,
    ToggleMockServer, ToggleSidebar,
};
use gpui::KeyBinding;

pub(crate) const SEND_KEYS: &str = "secondary-enter";
pub(super) const SAVE_KEYS: &str = "secondary-s";
pub(super) const SIDEBAR_KEYS: &str = "secondary-b";
pub(super) const MOCK_SERVER_KEYS: &str = "secondary-shift-m";
pub(super) const CONSOLE_KEYS: &str = "secondary-shift-c";
pub(super) const DOCS_KEYS: &str = "secondary-shift-d";
pub(super) const HELP_KEYS: &str = "f1";
pub(super) const ABOUT_KEYS: &str = "secondary-shift-a";
pub(super) const DISMISS_KEYS: &str = "escape";
pub(super) const QUIT_KEYS: &str = "secondary-q";

const SECONDARY_LABEL: &str = if cfg!(target_os = "macos") {
    "Cmd"
} else {
    "Ctrl"
};

/// Turns a binding such as `secondary-shift-m` into `Cmd+Shift+M` on macOS
/// and `Ctrl+Shift+M` elsewhere.
pub(crate) fn shortcut_label(keys: &str) -> String {
    keys.split('-')
        .map(|part| {
            if part == "secondary" {
                return SECONDARY_LABEL.to_string();
            }
            let mut chars = part.chars();
            chars
                .next()
                .map(|first| first.to_uppercase().chain(chars).collect())
                .unwrap_or_default()
        })
        .collect::<Vec<String>>()
        .join("+")
}

pub fn register_keybindings(cx: &mut gpui::App) {
    cx.bind_keys([
        KeyBinding::new(SEND_KEYS, SendRequest, None),
        KeyBinding::new(SAVE_KEYS, SaveRequest, None),
        KeyBinding::new(SIDEBAR_KEYS, ToggleSidebar, None),
        KeyBinding::new(MOCK_SERVER_KEYS, ToggleMockServer, None),
        KeyBinding::new(CONSOLE_KEYS, ToggleConsole, None),
        KeyBinding::new(DOCS_KEYS, ToggleDocs, None),
        KeyBinding::new(HELP_KEYS, ShowHelp, None),
        KeyBinding::new(ABOUT_KEYS, ShowAbout, None),
        KeyBinding::new(DISMISS_KEYS, DismissOverlay, None),
        KeyBinding::new(QUIT_KEYS, Quit, None),
    ]);
}

#[cfg(test)]
mod tests {
    use super::shortcut_label;

    #[test]
    fn the_secondary_modifier_is_labelled_for_the_platform() {
        let expected = if cfg!(target_os = "macos") {
            "Cmd"
        } else {
            "Ctrl"
        };
        assert_eq!(
            shortcut_label("secondary-shift-m"),
            format!("{expected}+Shift+M")
        );
    }

    #[test]
    fn plain_keys_are_capitalised() {
        assert_eq!(shortcut_label("f1"), "F1");
        assert_eq!(shortcut_label("escape"), "Escape");
    }

    #[test]
    fn every_global_binding_parses() {
        for keys in [
            super::SEND_KEYS,
            super::SAVE_KEYS,
            super::SIDEBAR_KEYS,
            super::MOCK_SERVER_KEYS,
            super::CONSOLE_KEYS,
            super::DOCS_KEYS,
            super::HELP_KEYS,
            super::ABOUT_KEYS,
            super::DISMISS_KEYS,
            super::QUIT_KEYS,
        ] {
            assert!(gpui::Keystroke::parse(keys).is_ok(), "{keys} must parse");
        }
    }
}
