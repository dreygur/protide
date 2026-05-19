use gpui::{ClipboardItem, Context, KeyDownEvent};
use super::*;

impl<E: WebSocketExecutor> RequestPanel<E> {
    pub(super) fn handle_edit_key(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let Some(target) = self.active_edit else { return; };
        let key = event.keystroke.key.as_str();
        let ctrl = event.keystroke.modifiers.control;
        let shift = event.keystroke.modifiers.shift;
        let is_body = matches!(target, EditTarget::Body);

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
                            let ins = if is_body { text.to_string() } else { text.replace('\n', "") };
                            self.edit_insert_text(&ins, cx);
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
            "up" => {
                if is_body {
                    let p = self.body_cursor_up();
                    if shift { self.edit_selection.end = p; cx.notify(); } else { self.edit_move_to(p, cx); }
                }
            }
            "down" => {
                if is_body {
                    let p = self.body_cursor_down();
                    if shift { self.edit_selection.end = p; cx.notify(); } else { self.edit_move_to(p, cx); }
                }
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
            "enter" => {
                if is_body { self.edit_insert_text("\n", cx); }
                else { self.move_to_next_field(cx); }
            }
            "tab" => {
                if is_body { self.edit_insert_text("  ", cx); }
                else { self.move_to_next_field(cx); }
            }
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

    pub(super) fn body_cursor_up(&self) -> usize {
        let text = &self.body;
        let cursor_char = self.edit_cursor();
        if text.is_empty() || cursor_char == 0 { return 0; }
        let cursor_byte = char_to_byte_offset(text, cursor_char);
        let line_start_byte = text[..cursor_byte].rfind('\n').map(|i| i + 1).unwrap_or(0);
        if line_start_byte == 0 { return 0; }
        let col = text[line_start_byte..cursor_byte].chars().count();
        let prev_end_byte = line_start_byte - 1;
        let prev_start_byte = text[..prev_end_byte].rfind('\n').map(|i| i + 1).unwrap_or(0);
        let prev_line_len = text[prev_start_byte..prev_end_byte].chars().count();
        text[..prev_start_byte].chars().count() + col.min(prev_line_len)
    }

    pub(super) fn body_cursor_down(&self) -> usize {
        let text = &self.body;
        let cursor_char = self.edit_cursor();
        if text.is_empty() { return 0; }
        let cursor_byte = char_to_byte_offset(text, cursor_char);
        let line_start_byte = text[..cursor_byte].rfind('\n').map(|i| i + 1).unwrap_or(0);
        let col = text[line_start_byte..cursor_byte].chars().count();
        let Some(nl) = text[cursor_byte..].find('\n') else { return text.chars().count(); };
        let next_start_byte = cursor_byte + nl + 1;
        let next_end_byte = text[next_start_byte..].find('\n').map(|i| next_start_byte + i).unwrap_or(text.len());
        let next_line_len = text[next_start_byte..next_end_byte].chars().count();
        text[..next_start_byte].chars().count() + col.min(next_line_len)
    }
}
