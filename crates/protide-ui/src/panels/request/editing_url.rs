use gpui::{ClipboardItem, Context, KeyDownEvent, MouseDownEvent, MouseMoveEvent, MouseUpEvent};
use super::*;
use crate::components::{find_word_start, find_word_end};

impl<E: WebSocketExecutor> RequestPanel<E> {
    pub(super) fn cursor(&self) -> usize {
        self.url_selection.end
    }

    pub(super) fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        let char_count = self.url.chars().count();
        let offset = offset.min(char_count);
        self.url_selection = offset..offset;
        cx.notify();
    }

    pub(super) fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        let char_count = self.url.chars().count();
        let offset = offset.min(char_count);
        self.url_selection.end = offset;
        if self.url_selection.end < self.url_selection.start {
            self.url_selection = self.url_selection.end..self.url_selection.start;
        }
        cx.notify();
    }

    pub(super) fn select_all(&mut self, cx: &mut Context<Self>) {
        self.url_selection = 0..self.url.chars().count();
        cx.notify();
    }

    pub(super) fn has_selection(&self) -> bool {
        self.url_selection.start != self.url_selection.end
    }

    pub(super) fn selected_text(&self) -> String {
        let lo = self.url_selection.start.min(self.url_selection.end);
        let hi = self.url_selection.start.max(self.url_selection.end);
        let byte_lo = char_to_byte_offset(&self.url, lo);
        let byte_hi = char_to_byte_offset(&self.url, hi);
        self.url[byte_lo..byte_hi].to_string()
    }

    pub(super) fn delete_selection(&mut self, cx: &mut Context<Self>) {
        if self.has_selection() {
            self.save_url_state();
            self.delete_selection_no_save(cx);
        }
    }

    pub(super) fn delete_selection_no_save(&mut self, cx: &mut Context<Self>) {
        if self.has_selection() {
            let char_start = self.url_selection.start.min(self.url_selection.end);
            let char_end = self.url_selection.start.max(self.url_selection.end);
            let byte_start = char_to_byte_offset(&self.url, char_start);
            let byte_end = char_to_byte_offset(&self.url, char_end);
            self.url.replace_range(byte_start..byte_end, "");
            self.url_selection = char_start..char_start;
            self.sync_params_from_url(cx);
            cx.notify();
        }
    }

    pub(super) fn save_url_state(&mut self) {
        self.url_undo_stack.push_back((self.url.clone(), self.url_selection.clone()));
        if self.url_undo_stack.len() > 100 {
            self.url_undo_stack.pop_front();
        }
        self.url_redo_stack.clear();
    }

    pub(super) fn url_undo(&mut self, cx: &mut Context<Self>) {
        if let Some((text, selection)) = self.url_undo_stack.pop_back() {
            self.url_redo_stack.push_back((self.url.clone(), self.url_selection.clone()));
            self.url = text;
            self.url_selection = selection;
            self.sync_params_from_url(cx);
            cx.notify();
        }
    }

    pub(super) fn url_redo(&mut self, cx: &mut Context<Self>) {
        if let Some((text, selection)) = self.url_redo_stack.pop_back() {
            self.url_undo_stack.push_back((self.url.clone(), self.url_selection.clone()));
            self.url = text;
            self.url_selection = selection;
            self.sync_params_from_url(cx);
            cx.notify();
        }
    }

    pub(super) fn insert_text(&mut self, text: &str, cx: &mut Context<Self>) {
        self.save_url_state();
        self.delete_selection_no_save(cx);
        let char_pos = self.url_selection.start;
        let byte_pos = char_to_byte_offset(&self.url, char_pos);
        self.url.insert_str(byte_pos, text);
        let new_char_pos = char_pos + text.chars().count();
        self.url_selection = new_char_pos..new_char_pos;
        self.sync_params_from_url(cx);
        cx.notify();
    }

    pub(super) fn index_for_x(&self, x: f32) -> usize {
        let char_width: f32 = 7.8;
        if x <= 0.0 { 0 } else { ((x / char_width) as usize).min(self.url.chars().count()) }
    }

    pub(super) fn handle_url_mouse_down(&mut self, event: &MouseDownEvent, cx: &mut Context<Self>) {
        self.is_selecting = true;
        let click_x = (f32::from(event.position.x) - self.url_input_left).max(0.0);
        let index = self.index_for_x(click_x);

        let effective_click = if event.click_count >= 4 { 1 } else { event.click_count };
        match effective_click {
            2 => {
                let start = find_word_start(&self.url, index);
                let end = find_word_end(&self.url, index);
                self.url_selection = start..end;
                cx.notify();
            }
            3 => { self.select_all(cx); }
            _ => {
                if event.modifiers.shift { self.select_to(index, cx); }
                else { self.move_to(index, cx); }
            }
        }
    }

    pub(super) fn handle_url_mouse_move(&mut self, event: &MouseMoveEvent, cx: &mut Context<Self>) {
        if self.is_selecting {
            let click_x = (f32::from(event.position.x) - self.url_input_left).max(0.0);
            let index = self.index_for_x(click_x);
            self.url_selection.end = index.min(self.url.chars().count());
            cx.notify();
        }
    }

    pub(super) fn handle_url_mouse_up(&mut self, _event: &MouseUpEvent, _cx: &mut Context<Self>) {
        self.is_selecting = false;
    }

    pub(super) fn handle_url_key(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let key = event.keystroke.key.as_str();
        let ctrl = event.keystroke.modifiers.control;
        let shift = event.keystroke.modifiers.shift;

        if ctrl {
            match key {
                "enter" => { self.send_request(cx); return; }
                "a" => { self.select_all(cx); return; }
                "c" => {
                    if self.has_selection() {
                        cx.write_to_clipboard(ClipboardItem::new_string(self.selected_text().to_string()));
                    }
                    return;
                }
                "x" => {
                    if self.has_selection() {
                        cx.write_to_clipboard(ClipboardItem::new_string(self.selected_text().to_string()));
                        self.delete_selection(cx);
                    }
                    return;
                }
                "v" => {
                    if let Some(item) = cx.read_from_clipboard() {
                        if let Some(text) = item.text() {
                            self.insert_text(&text.replace('\n', ""), cx);
                        }
                    }
                    return;
                }
                "z" => {
                    if shift { self.url_redo(cx); } else { self.url_undo(cx); }
                    return;
                }
                "y" => { self.url_redo(cx); return; }
                _ => {}
            }
        }

        match key {
            "left" => {
                if shift {
                    if self.url_selection.end > 0 { self.url_selection.end -= 1; cx.notify(); }
                } else if self.has_selection() {
                    let start = self.url_selection.start.min(self.url_selection.end);
                    self.move_to(start, cx);
                } else if self.cursor() > 0 {
                    self.move_to(self.cursor() - 1, cx);
                }
            }
            "right" => {
                let char_count = self.url.chars().count();
                if shift {
                    if self.url_selection.end < char_count { self.url_selection.end += 1; cx.notify(); }
                } else if self.has_selection() {
                    let end = self.url_selection.start.max(self.url_selection.end);
                    self.move_to(end, cx);
                } else if self.cursor() < char_count {
                    self.move_to(self.cursor() + 1, cx);
                }
            }
            "home" => {
                if shift { self.url_selection.end = 0; cx.notify(); }
                else { self.move_to(0, cx); }
            }
            "end" => {
                let char_count = self.url.chars().count();
                if shift { self.url_selection.end = char_count; cx.notify(); }
                else { self.move_to(char_count, cx); }
            }
            "backspace" => {
                if self.has_selection() {
                    self.delete_selection(cx);
                } else if self.cursor() > 0 {
                    self.save_url_state();
                    let char_pos = self.cursor() - 1;
                    let byte_pos = char_to_byte_offset(&self.url, char_pos);
                    let next_byte_pos = char_to_byte_offset(&self.url, char_pos + 1);
                    self.url.replace_range(byte_pos..next_byte_pos, "");
                    self.url_selection = char_pos..char_pos;
                    self.sync_params_from_url(cx);
                    cx.notify();
                }
            }
            "delete" => {
                let char_count = self.url.chars().count();
                if self.has_selection() {
                    self.delete_selection(cx);
                } else if self.cursor() < char_count {
                    self.save_url_state();
                    let cursor = self.cursor();
                    let byte_pos = char_to_byte_offset(&self.url, cursor);
                    let next_byte_pos = char_to_byte_offset(&self.url, cursor + 1);
                    self.url.replace_range(byte_pos..next_byte_pos, "");
                    self.sync_params_from_url(cx);
                    cx.notify();
                }
            }
            "enter" => { self.send_request(cx); }
            _ => {
                if let Some(ch) = &event.keystroke.key_char {
                    self.insert_text(ch, cx);
                }
            }
        }
        self.update_url_scroll();
    }

    pub(super) fn update_url_scroll(&mut self) {
        let char_width = 13.0 * 0.6;
        let padding = 14.0 * 2.0;
        let visible_width = (self.url_input_width - padding).max(60.0);
        let cursor_px = self.url_selection.end as f32 * char_width;

        if cursor_px < self.url_scroll_offset {
            self.url_scroll_offset = cursor_px;
        } else if cursor_px > self.url_scroll_offset + visible_width - char_width {
            self.url_scroll_offset = cursor_px - visible_width + char_width;
        }
        if self.url_scroll_offset < 0.0 {
            self.url_scroll_offset = 0.0;
        }
    }
}
