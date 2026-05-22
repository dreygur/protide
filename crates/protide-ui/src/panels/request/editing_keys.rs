use gpui::{ClipboardItem, Context, KeyDownEvent};
use super::*;

impl<E: WebSocketExecutor> RequestPanel<E> {
    pub(super) fn handle_edit_key(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let Some(target) = self.active_edit else { return; };
        let key = event.keystroke.key.as_str();
        let ctrl = event.keystroke.modifiers.control;
        let shift = event.keystroke.modifiers.shift;

        if ctrl {
            match key {
                "a" => { self.edit_select_all(cx); return; }
                "c" => {
                    if self.edit_has_selection() {
                        cx.write_to_clipboard(ClipboardItem::new_string(self.edit_selected_text()));
                    }
                    return;
                }
                "x" => {
                    if self.edit_has_selection() {
                        cx.write_to_clipboard(ClipboardItem::new_string(self.edit_selected_text()));
                        self.edit_delete_selection(cx);
                    }
                    return;
                }
                "v" => {
                    if let Some(item) = cx.read_from_clipboard() {
                        if let Some(text) = item.text() {
                            self.edit_insert_text(&text.replace('\n', ""), cx);
                        }
                    }
                    return;
                }
                "z" => { if shift { self.edit_redo(cx); } else { self.edit_undo(cx); } return; }
                "y" => { self.edit_redo(cx); return; }
                _ => {}
            }
        }

        match key {
            "left" => {
                if shift {
                    if self.edit_selection.end > 0 { self.edit_selection.end -= 1; cx.notify(); }
                } else if self.edit_has_selection() {
                    let s = self.edit_selection.start.min(self.edit_selection.end);
                    self.edit_move_to(s, cx);
                } else if self.edit_cursor() > 0 {
                    self.edit_move_to(self.edit_cursor() - 1, cx);
                }
            }
            "right" => {
                let n = self.get_edit_text(target).chars().count();
                if shift {
                    if self.edit_selection.end < n { self.edit_selection.end += 1; cx.notify(); }
                } else if self.edit_has_selection() {
                    let e = self.edit_selection.start.max(self.edit_selection.end);
                    self.edit_move_to(e, cx);
                } else if self.edit_cursor() < n {
                    self.edit_move_to(self.edit_cursor() + 1, cx);
                }
            }
            "home" => {
                if shift { self.edit_selection.end = 0; cx.notify(); }
                else { self.edit_move_to(0, cx); }
            }
            "end" => {
                let n = self.get_edit_text(target).chars().count();
                if shift { self.edit_selection.end = n; cx.notify(); }
                else { self.edit_move_to(n, cx); }
            }
            "backspace" => {
                if self.edit_has_selection() {
                    self.edit_delete_selection(cx);
                } else if self.edit_cursor() > 0 {
                    self.save_edit_state();
                    let char_pos = self.edit_cursor() - 1;
                    if let Some(text) = self.get_edit_text_mut(target) {
                        let b0 = char_to_byte_offset(text, char_pos);
                        let b1 = char_to_byte_offset(text, char_pos + 1);
                        text.replace_range(b0..b1, "");
                        self.edit_selection = char_pos..char_pos;
                        self.sync_after_edit(target, cx);
                        cx.notify();
                    }
                }
            }
            "delete" => {
                let n = self.get_edit_text(target).chars().count();
                if self.edit_has_selection() {
                    self.edit_delete_selection(cx);
                } else {
                    let c = self.edit_cursor();
                    if c < n {
                        self.save_edit_state();
                        if let Some(text) = self.get_edit_text_mut(target) {
                            let b0 = char_to_byte_offset(text, c);
                            let b1 = char_to_byte_offset(text, c + 1);
                            text.replace_range(b0..b1, "");
                            self.sync_after_edit(target, cx);
                            cx.notify();
                        }
                    }
                }
            }
            "escape" => { self.stop_editing(cx); }
            "enter" => { self.move_to_next_field(cx); }
            "tab" => { self.move_to_next_field(cx); }
            _ => {
                if let Some(ch) = &event.keystroke.key_char {
                    self.edit_insert_text(ch, cx);
                }
            }
        }
    }

    pub(super) fn move_to_next_field(&mut self, cx: &mut Context<Self>) {
        let Some(target) = self.active_edit else { return; };
        let next_target = match target {
            EditTarget::HeaderKey(i) => Some(EditTarget::HeaderValue(i)),
            EditTarget::HeaderValue(i) => {
                if i + 1 < self.headers.len() { Some(EditTarget::HeaderKey(i + 1)) } else { None }
            }
            EditTarget::ParamKey(i) => Some(EditTarget::ParamValue(i)),
            EditTarget::ParamValue(i) => {
                if i + 1 < self.params.len() { Some(EditTarget::ParamKey(i + 1)) } else { None }
            }
            EditTarget::FormKey(i) => Some(EditTarget::FormValue(i)),
            EditTarget::FormValue(i) => {
                if i + 1 < self.form_data.len() { Some(EditTarget::FormKey(i + 1)) } else { None }
            }
            _ => None,
        };
        if let Some(next) = next_target {
            let len = self.get_edit_text(next).len();
            self.active_edit = Some(next);
            self.edit_selection = len..len;
            cx.notify();
        } else {
            self.stop_editing(cx);
        }
    }
}
