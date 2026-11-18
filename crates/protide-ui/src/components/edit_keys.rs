//! Shortcut mapping shared by the hand rolled single line text inputs, so
//! they all follow the platform modifier (cmd on macOS, ctrl elsewhere).

use gpui::Keystroke;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EditCommand {
    SelectAll,
    Copy,
    Cut,
    Paste,
    Undo,
    Redo,
    LineStart,
    LineEnd,
}

/// The editing command a keystroke asks for. Shift is left to the caller,
/// which uses it to extend the selection on `LineStart` / `LineEnd`.
pub(crate) fn edit_command(keystroke: &Keystroke) -> Option<EditCommand> {
    let modifiers = &keystroke.modifiers;
    let key = keystroke.key.as_str();
    let mac = cfg!(target_os = "macos");
    if modifiers.secondary() {
        return match key {
            "a" => Some(EditCommand::SelectAll),
            "c" => Some(EditCommand::Copy),
            "x" => Some(EditCommand::Cut),
            "v" => Some(EditCommand::Paste),
            "z" if modifiers.shift => Some(EditCommand::Redo),
            "z" => Some(EditCommand::Undo),
            "y" => Some(EditCommand::Redo),
            "left" if mac => Some(EditCommand::LineStart),
            "right" if mac => Some(EditCommand::LineEnd),
            _ => None,
        };
    }
    match key {
        "home" => Some(EditCommand::LineStart),
        "end" => Some(EditCommand::LineEnd),
        _ => None,
    }
}

/// The text a keystroke types. A held ctrl or cmd means a shortcut, so an
/// unbound one never inserts its letter.
pub(crate) fn typed_text(keystroke: &Keystroke) -> Option<&str> {
    if keystroke.modifiers.control || keystroke.modifiers.platform {
        return None;
    }
    keystroke.key_char.as_deref()
}

#[cfg(test)]
mod tests {
    use super::{EditCommand, edit_command, typed_text};
    use gpui::Keystroke;

    fn command(keys: &str) -> Option<EditCommand> {
        edit_command(&Keystroke::parse(keys).unwrap())
    }

    #[test]
    fn clipboard_and_history_shortcuts_follow_the_platform_modifier() {
        assert_eq!(command("secondary-a"), Some(EditCommand::SelectAll));
        assert_eq!(command("secondary-c"), Some(EditCommand::Copy));
        assert_eq!(command("secondary-x"), Some(EditCommand::Cut));
        assert_eq!(command("secondary-v"), Some(EditCommand::Paste));
        assert_eq!(command("secondary-z"), Some(EditCommand::Undo));
        assert_eq!(command("secondary-shift-z"), Some(EditCommand::Redo));
        assert_eq!(command("secondary-y"), Some(EditCommand::Redo));
        assert_eq!(command("secondary-q"), None);
    }

    #[test]
    fn home_and_end_move_to_the_line_edges_with_or_without_shift() {
        assert_eq!(command("home"), Some(EditCommand::LineStart));
        assert_eq!(command("shift-end"), Some(EditCommand::LineEnd));
        assert_eq!(command("left"), None);
        assert_eq!(command("a"), None);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn on_macos_cmd_is_the_shortcut_modifier_and_ctrl_is_not() {
        assert_eq!(command("cmd-v"), Some(EditCommand::Paste));
        assert_eq!(command("cmd-left"), Some(EditCommand::LineStart));
        assert_eq!(command("cmd-shift-right"), Some(EditCommand::LineEnd));
        assert_eq!(command("ctrl-v"), None);
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn off_macos_ctrl_is_the_shortcut_modifier_and_ctrl_arrows_are_not_line_moves() {
        assert_eq!(command("ctrl-v"), Some(EditCommand::Paste));
        assert_eq!(command("ctrl-left"), None);
    }

    #[test]
    fn a_held_shortcut_modifier_types_nothing() {
        assert_eq!(typed_text(&Keystroke::parse("a->a").unwrap()), Some("a"));
        assert_eq!(
            typed_text(&Keystroke::parse("shift-a->A").unwrap()),
            Some("A")
        );
        assert_eq!(typed_text(&Keystroke::parse("cmd-q->q").unwrap()), None);
        assert_eq!(typed_text(&Keystroke::parse("ctrl-q->q").unwrap()), None);
    }
}
