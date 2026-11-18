use crate::components::edit_keys::{EditCommand, edit_command, typed_text};
use gpui::{ClipboardItem, Context, KeyDownEvent};
use std::ops::Range;

fn char_to_byte(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(b, _)| b)
        .unwrap_or(s.len())
}

pub(super) fn ordered(selection: &Range<usize>) -> Range<usize> {
    selection.start.min(selection.end)..selection.start.max(selection.end)
}

/// The bytes of `s` a char based selection covers, in either direction.
pub(super) fn selected_bytes(s: &str, selection: &Range<usize>) -> Range<usize> {
    let chars = ordered(selection);
    char_to_byte(s, chars.start)..char_to_byte(s, chars.end)
}

use super::*;

impl ExplorerPanel {
    fn rename_replace_selection(&mut self, insert: &str, cx: &mut Context<Self>) {
        let start = ordered(&self.rename_selection).start;
        let bytes = selected_bytes(&self.rename_text, &self.rename_selection);
        self.rename_text.replace_range(bytes, insert);
        let pos = start + insert.chars().count();
        self.rename_selection = pos..pos;
        cx.notify();
    }

    fn handle_rename_command(&mut self, command: EditCommand, shift: bool, cx: &mut Context<Self>) {
        match command {
            EditCommand::SelectAll => {
                self.rename_selection = 0..self.rename_text.chars().count();
            }
            EditCommand::Copy | EditCommand::Cut => {
                let bytes = selected_bytes(&self.rename_text, &self.rename_selection);
                if bytes.is_empty() {
                    return;
                }
                let copied = self.rename_text[bytes].to_string();
                cx.write_to_clipboard(ClipboardItem::new_string(copied));
                if command == EditCommand::Cut {
                    self.rename_replace_selection("", cx);
                }
            }
            EditCommand::Paste => {
                if let Some(item) = cx.read_from_clipboard()
                    && let Some(paste) = item.text()
                {
                    self.rename_replace_selection(&paste.replace('\n', ""), cx);
                }
            }
            EditCommand::Undo | EditCommand::Redo => {}
            EditCommand::LineStart | EditCommand::LineEnd => {
                let edge = if command == EditCommand::LineStart {
                    0
                } else {
                    self.rename_text.chars().count()
                };
                self.rename_selection.end = edge;
                if !shift {
                    self.rename_selection.start = edge;
                }
            }
        }
        cx.notify();
    }

    fn handle_rename_key(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let shift = event.keystroke.modifiers.shift;
        if let Some(command) = edit_command(&event.keystroke) {
            self.handle_rename_command(command, shift, cx);
            return;
        }
        let selection = ordered(&self.rename_selection);

        match event.keystroke.key.as_str() {
            "escape" => self.cancel_rename(cx),
            "enter" => self.complete_rename(cx),
            "backspace" => {
                if !selection.is_empty() {
                    self.rename_replace_selection("", cx);
                } else if selection.start > 0 {
                    self.rename_selection = (selection.start - 1)..selection.start;
                    self.rename_replace_selection("", cx);
                }
            }
            "left" => {
                if !selection.is_empty() && !shift {
                    self.rename_selection = selection.start..selection.start;
                } else if self.rename_selection.end > 0 {
                    self.rename_selection.end -= 1;
                    if !shift {
                        self.rename_selection.start = self.rename_selection.end;
                    }
                }
                cx.notify();
            }
            "right" => {
                let n = self.rename_text.chars().count();
                if !selection.is_empty() && !shift {
                    self.rename_selection = selection.end..selection.end;
                } else if self.rename_selection.end < n {
                    self.rename_selection.end += 1;
                    if !shift {
                        self.rename_selection.start = self.rename_selection.end;
                    }
                }
                cx.notify();
            }
            _ => {
                if let Some(text) = typed_text(&event.keystroke) {
                    self.rename_replace_selection(text, cx);
                }
            }
        }
    }

    fn handle_env_command(
        &mut self,
        target: EnvEditTarget,
        command: EditCommand,
        shift: bool,
        cx: &mut Context<Self>,
    ) {
        match command {
            EditCommand::SelectAll => {
                self.edit_selection = 0..self.get_edit_text(target).chars().count();
            }
            EditCommand::Copy | EditCommand::Cut => {
                let text = self.get_edit_text(target);
                let bytes = selected_bytes(&text, &self.edit_selection);
                if bytes.is_empty() {
                    return;
                }
                cx.write_to_clipboard(ClipboardItem::new_string(text[bytes].to_string()));
                if command == EditCommand::Cut {
                    self.insert_text(target, "", cx);
                }
            }
            EditCommand::Paste => {
                if let Some(item) = cx.read_from_clipboard()
                    && let Some(paste) = item.text()
                {
                    self.insert_text(target, &paste.replace('\n', ""), cx);
                }
            }
            EditCommand::Undo => self.edit_undo(cx),
            EditCommand::Redo => self.edit_redo(cx),
            EditCommand::LineStart | EditCommand::LineEnd => {
                let edge = if command == EditCommand::LineStart {
                    0
                } else {
                    self.get_edit_text(target).chars().count()
                };
                self.edit_selection.end = edge;
                if !shift {
                    self.edit_selection.start = edge;
                }
                self.update_env_scroll(target);
            }
        }
        cx.notify();
    }

    pub(super) fn handle_edit_key(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        if self.renaming_item.is_some() {
            self.handle_rename_key(event, cx);
            return;
        }
        let Some(target) = self.active_edit else {
            return;
        };
        let shift = event.keystroke.modifiers.shift;
        if let Some(command) = edit_command(&event.keystroke) {
            self.handle_env_command(target, command, shift, cx);
            return;
        }

        match event.keystroke.key.as_str() {
            "escape" => {
                if target == EnvEditTarget::NewEnvName {
                    self.cancel_new_env(cx);
                } else {
                    self.stop_editing(cx);
                }
            }
            "enter" => {
                if target == EnvEditTarget::NewEnvName {
                    self.create_new_env(cx);
                } else {
                    self.stop_editing(cx);
                }
            }
            "backspace" => {
                let end = self.edit_selection.end;
                if self.edit_selection.start == end && end > 0 {
                    self.edit_selection = (end - 1)..end;
                }
                if self.edit_selection.start != self.edit_selection.end {
                    self.insert_text(target, "", cx);
                }
            }
            "left" => {
                if self.edit_selection.end > 0 {
                    self.edit_selection.end -= 1;
                    if !shift {
                        self.edit_selection.start = self.edit_selection.end;
                    }
                    self.update_env_scroll(target);
                    cx.notify();
                }
            }
            "right" => {
                let text_len = self.get_edit_text(target).chars().count();
                if self.edit_selection.end < text_len {
                    self.edit_selection.end += 1;
                    if !shift {
                        self.edit_selection.start = self.edit_selection.end;
                    }
                    self.update_env_scroll(target);
                    cx.notify();
                }
            }
            _ => {
                if let Some(text) = typed_text(&event.keystroke) {
                    self.insert_text(target, text, cx);
                }
            }
        }
    }
}
