use gpui::{ClipboardItem, Context, KeyDownEvent, MouseDownEvent, MouseMoveEvent, MouseUpEvent};
use super::*;

impl ExplorerPanel {
    pub(super) fn handle_edit_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        target: EnvEditTarget,
        char_width: f32,
        cx: &mut Context<Self>,
    ) {
        self.edit_is_selecting = true;
        let text_start_x = self.edit_input_origins.get(&target).copied().unwrap_or(0.0);
        let click_x = (f32::from(event.position.x) - text_start_x).max(0.0);
        let index = self.edit_index_for_x(click_x.max(0.0), char_width);

        let effective_click = if event.click_count >= 4 { 1 } else { event.click_count };

        match effective_click {
            2 => {
                let text = self.get_edit_text(target);
                let start = crate::ui::components::find_word_start(&text, index);
                let end = crate::ui::components::find_word_end(&text, index);
                self.edit_selection = start..end;
                cx.notify();
            }
            3 => {
                let text_len = self.get_edit_text(target).chars().count();
                self.edit_selection = 0..text_len;
                cx.notify();
            }
            _ => {
                self.edit_selection = index..index;
                cx.notify();
            }
        }
    }

    pub(super) fn handle_edit_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        char_width: f32,
        cx: &mut Context<Self>,
    ) {
        if self.edit_is_selecting {
            let text_start_x = self
                .active_edit
                .and_then(|t| self.edit_input_origins.get(&t).copied())
                .unwrap_or(0.0);
            let click_x = f32::from(event.position.x) - text_start_x;
            let index = self.edit_index_for_x(click_x.max(0.0), char_width);
            self.edit_selection.end = index;
            cx.notify();
        }
    }

    pub(super) fn handle_edit_mouse_up(&mut self, _event: &MouseUpEvent, _cx: &mut Context<Self>) {
        self.edit_is_selecting = false;
    }

    pub(super) fn handle_edit_key(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        if self.renaming_item.is_some() {
            let key = event.keystroke.key.as_str();
            match key {
                "escape" => { self.cancel_rename(cx); return; }
                "enter" => { self.complete_rename(cx); return; }
                "backspace" => { self.rename_text.pop(); cx.notify(); return; }
                _ if key.len() == 1 && !event.keystroke.modifiers.control => {
                    self.rename_text.push_str(key);
                    cx.notify();
                    return;
                }
                _ => return,
            }
        }

        let Some(target) = self.active_edit else { return; };
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
                        cx.write_to_clipboard(ClipboardItem::new_string(text[start..end].to_string()));
                    }
                    return;
                }
                "v" => {
                    if let Some(item) = cx.read_from_clipboard()
                        && let Some(paste_text) = item.text() {
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
                "y" => { self.edit_redo(cx); return; }
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
                    self.set_edit_text(target, text);
                    self.update_env_scroll(target);
                    cx.notify();
                } else if self.edit_selection.end > 0 {
                    self.save_edit_state();
                    let pos = self.edit_selection.end - 1;
                    text.remove(pos);
                    self.edit_selection = pos..pos;
                    self.set_edit_text(target, text);
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
