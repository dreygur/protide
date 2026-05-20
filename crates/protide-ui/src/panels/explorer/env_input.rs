use gpui::{ClipboardItem, Context, KeyDownEvent, MouseDownEvent, MouseMoveEvent, MouseUpEvent};

fn char_to_byte(s: &str, char_idx: usize) -> usize {
    s.char_indices().nth(char_idx).map(|(b, _)| b).unwrap_or(s.len())
}
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
                let start = crate::components::find_word_start(&text, index);
                let end = crate::components::find_word_end(&text, index);
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

    pub(super) fn handle_rename_mouse_down(&mut self, event: &MouseDownEvent, cx: &mut Context<Self>) {
        let click_x = (f32::from(event.position.x) - self.rename_input_origin).max(0.0);
        let idx = ((click_x / 7.2) as usize).min(self.rename_text.chars().count());
        match event.click_count {
            2 => {
                let start = crate::components::find_word_start(&self.rename_text, idx);
                let end = crate::components::find_word_end(&self.rename_text, idx);
                self.rename_selection = start..end;
            }
            3 => {
                let n = self.rename_text.chars().count();
                self.rename_selection = 0..n;
            }
            _ => { self.rename_selection = idx..idx; }
        }
        self.rename_is_selecting = true;
        cx.notify();
    }

    pub(super) fn handle_rename_mouse_move(&mut self, event: &MouseMoveEvent, cx: &mut Context<Self>) {
        if self.rename_is_selecting {
            let click_x = (f32::from(event.position.x) - self.rename_input_origin).max(0.0);
            let idx = ((click_x / 7.2) as usize).min(self.rename_text.chars().count());
            if self.rename_selection.end != idx {
                self.rename_selection.end = idx;
                cx.notify();
            }
        }
    }

    pub(super) fn handle_edit_key(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        if self.renaming_item.is_some() {
            let key = event.keystroke.key.as_str();
            let ctrl = event.keystroke.modifiers.control;
            let shift = event.keystroke.modifiers.shift;
            let sel_start = self.rename_selection.start.min(self.rename_selection.end);
            let sel_end = self.rename_selection.start.max(self.rename_selection.end);

            match key {
                "escape" => { self.cancel_rename(cx); return; }
                "enter" => { self.complete_rename(cx); return; }
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
                    cx.notify(); return;
                }
                "left" => {
                    if sel_start != sel_end && !shift {
                        self.rename_selection = sel_start..sel_start;
                    } else if self.rename_selection.end > 0 {
                        self.rename_selection.end -= 1;
                        if !shift { self.rename_selection.start = self.rename_selection.end; }
                    }
                    cx.notify(); return;
                }
                "right" => {
                    let n = self.rename_text.chars().count();
                    if sel_start != sel_end && !shift {
                        self.rename_selection = sel_end..sel_end;
                    } else if self.rename_selection.end < n {
                        self.rename_selection.end += 1;
                        if !shift { self.rename_selection.start = self.rename_selection.end; }
                    }
                    cx.notify(); return;
                }
                "home" => {
                    self.rename_selection.end = 0;
                    if !shift { self.rename_selection.start = 0; }
                    cx.notify(); return;
                }
                "end" => {
                    let n = self.rename_text.chars().count();
                    self.rename_selection.end = n;
                    if !shift { self.rename_selection.start = n; }
                    cx.notify(); return;
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
                                self.rename_text[bs..be].to_string()
                            ));
                        }
                        "v" => {
                            if let Some(item) = cx.read_from_clipboard()
                                && let Some(paste) = item.text() {
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
