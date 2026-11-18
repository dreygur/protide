use gpui::{ClipboardItem, Context, KeyDownEvent};

fn char_to_byte(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(b, _)| b)
        .unwrap_or(s.len())
}
use super::*;

impl ExplorerPanel {
    pub(super) fn handle_edit_key(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        if self.renaming_item.is_some() {
            let key = event.keystroke.key.as_str();
            let ctrl = event.keystroke.modifiers.control;
            let shift = event.keystroke.modifiers.shift;
            let sel_start = self.rename_selection.start.min(self.rename_selection.end);
            let sel_end = self.rename_selection.start.max(self.rename_selection.end);

            match key {
                "escape" => {
                    self.cancel_rename(cx);
                    return;
                }
                "enter" => {
                    self.complete_rename(cx);
                    return;
                }
                "backspace" => {
                    if sel_start != sel_end {
                        let bs = char_to_byte(&self.rename_text, sel_start);
                        let be = char_to_byte(&self.rename_text, sel_end);
                        self.rename_text.replace_range(bs..be, "");
                        self.rename_selection = sel_start..sel_start;
                    } else if sel_start > 0 {
                        let bs = char_to_byte(&self.rename_text, sel_start - 1);
                        let be = char_to_byte(&self.rename_text, sel_start);
                        self.rename_text.replace_range(bs..be, "");
                        self.rename_selection = (sel_start - 1)..(sel_start - 1);
                    }
                    cx.notify();
                    return;
                }
                "left" => {
                    if sel_start != sel_end && !shift {
                        self.rename_selection = sel_start..sel_start;
                    } else if self.rename_selection.end > 0 {
                        self.rename_selection.end -= 1;
                        if !shift {
                            self.rename_selection.start = self.rename_selection.end;
                        }
                    }
                    cx.notify();
                    return;
                }
                "right" => {
                    let n = self.rename_text.chars().count();
                    if sel_start != sel_end && !shift {
                        self.rename_selection = sel_end..sel_end;
                    } else if self.rename_selection.end < n {
                        self.rename_selection.end += 1;
                        if !shift {
                            self.rename_selection.start = self.rename_selection.end;
                        }
                    }
                    cx.notify();
                    return;
                }
                "home" => {
                    self.rename_selection.end = 0;
                    if !shift {
                        self.rename_selection.start = 0;
                    }
                    cx.notify();
                    return;
                }
                "end" => {
                    let n = self.rename_text.chars().count();
                    self.rename_selection.end = n;
                    if !shift {
                        self.rename_selection.start = n;
                    }
                    cx.notify();
                    return;
                }
                _ if ctrl => {
                    match key {
                        "a" => {
                            let n = self.rename_text.chars().count();
                            self.rename_selection = 0..n;
                            cx.notify();
                        }
                        "c" if sel_start != sel_end => {
                            let bs = char_to_byte(&self.rename_text, sel_start);
                            let be = char_to_byte(&self.rename_text, sel_end);
                            cx.write_to_clipboard(ClipboardItem::new_string(
                                self.rename_text[bs..be].to_string(),
                            ));
                        }
                        "v" => {
                            if let Some(item) = cx.read_from_clipboard()
                                && let Some(paste) = item.text()
                            {
                                let paste = paste.replace('\n', "");
                                if sel_start != sel_end {
                                    let bs = char_to_byte(&self.rename_text, sel_start);
                                    let be = char_to_byte(&self.rename_text, sel_end);
                                    self.rename_text.replace_range(bs..be, "");
                                }
                                let byte_pos = char_to_byte(&self.rename_text, sel_start);
                                self.rename_text.insert_str(byte_pos, &paste);
                                let new_pos = sel_start + paste.chars().count();
                                self.rename_selection = new_pos..new_pos;
                                cx.notify();
                            }
                        }
                        _ => {}
                    }
                    return;
                }
                _ => {
                    if let Some(ch) = &event.keystroke.key_char {
                        if sel_start != sel_end {
                            let bs = char_to_byte(&self.rename_text, sel_start);
                            let be = char_to_byte(&self.rename_text, sel_end);
                            self.rename_text.replace_range(bs..be, "");
                        }
                        let byte_pos = char_to_byte(&self.rename_text, sel_start);
                        self.rename_text.insert_str(byte_pos, ch.as_str());
                        let new_pos = sel_start + ch.chars().count();
                        self.rename_selection = new_pos..new_pos;
                        cx.notify();
                    }
                    return;
                }
            }
        }

        let Some(target) = self.active_edit else {
            return;
        };
        let key = event.keystroke.key.as_str();
        let ctrl = event.keystroke.modifiers.control;

        if ctrl {
            match key {
                "a" => {
                    let text_len = self.get_edit_text(target).len();
                    self.edit_selection = 0..text_len;
                    cx.notify();
                    return;
                }
                "c" => {
                    if self.edit_selection.start != self.edit_selection.end {
                        let text = self.get_edit_text(target);
                        let start = self.edit_selection.start.min(self.edit_selection.end);
                        let end = self.edit_selection.start.max(self.edit_selection.end);
                        cx.write_to_clipboard(ClipboardItem::new_string(
                            text[start..end].to_string(),
                        ));
                    }
                    return;
                }
                "v" => {
                    if let Some(item) = cx.read_from_clipboard()
                        && let Some(paste_text) = item.text()
                    {
                        let paste_text = paste_text.replace('\n', "");
                        self.insert_text(target, &paste_text, cx);
                    }
                    return;
                }
                "z" => {
                    if event.keystroke.modifiers.shift {
                        self.edit_redo(cx);
                    } else {
                        self.edit_undo(cx);
                    }
                    return;
                }
                "y" => {
                    self.edit_redo(cx);
                    return;
                }
                _ => {}
            }
        }

        match key {
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
                let mut text = self.get_edit_text(target);
                if self.edit_selection.start != self.edit_selection.end {
                    self.save_edit_state();
                    let start = self.edit_selection.start.min(self.edit_selection.end);
                    let end = self.edit_selection.start.max(self.edit_selection.end);
                    text.replace_range(start..end, "");
                    self.edit_selection = start..start;
                    self.set_edit_text(target, text, cx);
                    self.update_env_scroll(target);
                    cx.notify();
                } else if self.edit_selection.end > 0 {
                    self.save_edit_state();
                    let pos = self.edit_selection.end - 1;
                    text.remove(pos);
                    self.edit_selection = pos..pos;
                    self.set_edit_text(target, text, cx);
                    self.update_env_scroll(target);
                    cx.notify();
                }
            }
            "left" => {
                if self.edit_selection.end > 0 {
                    self.edit_selection.end -= 1;
                    if !event.keystroke.modifiers.shift {
                        self.edit_selection.start = self.edit_selection.end;
                    }
                    self.update_env_scroll(target);
                    cx.notify();
                }
            }
            "right" => {
                let text_len = self.get_edit_text(target).len();
                if self.edit_selection.end < text_len {
                    self.edit_selection.end += 1;
                    if !event.keystroke.modifiers.shift {
                        self.edit_selection.start = self.edit_selection.end;
                    }
                    self.update_env_scroll(target);
                    cx.notify();
                }
            }
            _ => {
                if let Some(ch) = &event.keystroke.key_char {
                    self.insert_text(target, ch, cx);
                }
            }
        }
    }
}
