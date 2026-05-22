use gpui::{Context, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Window};
use super::*;
use crate::components::{find_word_start, find_word_end};

impl<E: WebSocketExecutor> RequestPanel<E> {
    pub(super) fn get_edit_text(&self, target: EditTarget) -> &str {
        match target {
            EditTarget::HeaderKey(i) => self.headers.get(i).map(|h| h.key.as_str()).unwrap_or(""),
            EditTarget::HeaderValue(i) => self.headers.get(i).map(|h| h.value.as_str()).unwrap_or(""),
            EditTarget::ParamKey(i) => self.params.get(i).map(|p| p.key.as_str()).unwrap_or(""),
            EditTarget::ParamValue(i) => self.params.get(i).map(|p| p.value.as_str()).unwrap_or(""),
            EditTarget::FormKey(i) => self.form_data.get(i).map(|f| f.key.as_str()).unwrap_or(""),
            EditTarget::FormValue(i) => self.form_data.get(i).map(|f| f.value.as_str()).unwrap_or(""),
            EditTarget::BearerToken => &self.bearer_token,
            EditTarget::BasicUsername => &self.basic_username,
            EditTarget::BasicPassword => &self.basic_password,
            EditTarget::ApiKeyName => &self.api_key_name,
            EditTarget::ApiKeyValue => &self.api_key_value,
            EditTarget::GrpcMetaKey(i) => self.grpc_metadata.get(i).map(|m| m.key.as_str()).unwrap_or(""),
            EditTarget::GrpcMetaValue(i) => self.grpc_metadata.get(i).map(|m| m.value.as_str()).unwrap_or(""),
            EditTarget::SioNamespace => &self.sio_namespace,
            EditTarget::SioEventName => &self.sio_event_name,
            EditTarget::SioRoomName => &self.sio_room_name,
        }
    }

    pub(super) fn get_edit_text_mut(&mut self, target: EditTarget) -> Option<&mut String> {
        match target {
            EditTarget::HeaderKey(i) => self.headers.get_mut(i).map(|h| &mut h.key),
            EditTarget::HeaderValue(i) => self.headers.get_mut(i).map(|h| &mut h.value),
            EditTarget::ParamKey(i) => self.params.get_mut(i).map(|p| &mut p.key),
            EditTarget::ParamValue(i) => self.params.get_mut(i).map(|p| &mut p.value),
            EditTarget::FormKey(i) => self.form_data.get_mut(i).map(|f| &mut f.key),
            EditTarget::FormValue(i) => self.form_data.get_mut(i).map(|f| &mut f.value),
            EditTarget::BearerToken => Some(&mut self.bearer_token),
            EditTarget::BasicUsername => Some(&mut self.basic_username),
            EditTarget::BasicPassword => Some(&mut self.basic_password),
            EditTarget::ApiKeyName => Some(&mut self.api_key_name),
            EditTarget::ApiKeyValue => Some(&mut self.api_key_value),
            EditTarget::GrpcMetaKey(i) => self.grpc_metadata.get_mut(i).map(|m| &mut m.key),
            EditTarget::GrpcMetaValue(i) => self.grpc_metadata.get_mut(i).map(|m| &mut m.value),
            EditTarget::SioNamespace => Some(&mut self.sio_namespace),
            EditTarget::SioEventName => Some(&mut self.sio_event_name),
            EditTarget::SioRoomName => Some(&mut self.sio_room_name),
        }
    }

    pub(super) fn start_editing(&mut self, target: EditTarget, window: &mut Window, cx: &mut Context<Self>) {
        let text_len = self.get_edit_text(target).chars().count();
        self.active_edit = Some(target);
        self.edit_selection = text_len..text_len;
        self.edit_is_selecting = false;
        self.edit_focus.focus(window, cx);
        self._edit_blur_sub = Some(cx.on_blur(&self.edit_focus, window, |this, _, cx| {
            this.stop_editing(cx);
        }));
        cx.notify();
    }

    pub(super) fn stop_editing(&mut self, cx: &mut Context<Self>) {
        self.active_edit = None;
        self.edit_selection = 0..0;
        self.edit_is_selecting = false;
        cx.notify();
    }

    pub(super) fn edit_cursor(&self) -> usize { self.edit_selection.end }
    pub(super) fn edit_has_selection(&self) -> bool { self.edit_selection.start != self.edit_selection.end }

    pub(super) fn edit_selected_text(&self) -> String {
        if let Some(target) = self.active_edit {
            let text = self.get_edit_text(target);
            let char_start = self.edit_selection.start.min(self.edit_selection.end);
            let char_end = self.edit_selection.start.max(self.edit_selection.end);
            let byte_start = char_to_byte_offset(text, char_start);
            let byte_end = char_to_byte_offset(text, char_end);
            text[byte_start..byte_end].to_string()
        } else { String::new() }
    }

    pub(super) fn edit_move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        if let Some(target) = self.active_edit {
            let offset = offset.min(self.get_edit_text(target).chars().count());
            self.edit_selection = offset..offset;
            cx.notify();
        }
    }

    pub(super) fn edit_select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        if let Some(target) = self.active_edit {
            self.edit_selection.end = offset.min(self.get_edit_text(target).chars().count());
            cx.notify();
        }
    }

    pub(super) fn edit_select_all(&mut self, cx: &mut Context<Self>) {
        if let Some(target) = self.active_edit {
            self.edit_selection = 0..self.get_edit_text(target).chars().count();
            cx.notify();
        }
    }

    pub(super) fn edit_delete_selection(&mut self, cx: &mut Context<Self>) {
        if self.active_edit.is_some() && self.edit_has_selection() {
            self.save_edit_state();
            self.edit_delete_selection_no_save(cx);
        }
    }

    pub(super) fn edit_delete_selection_no_save(&mut self, cx: &mut Context<Self>) {
        if let Some(target) = self.active_edit {
            if self.edit_has_selection() {
                let char_start = self.edit_selection.start.min(self.edit_selection.end);
                let char_end = self.edit_selection.start.max(self.edit_selection.end);
                if let Some(text) = self.get_edit_text_mut(target) {
                    let byte_start = char_to_byte_offset(text, char_start);
                    let byte_end = char_to_byte_offset(text, char_end);
                    text.replace_range(byte_start..byte_end, "");
                    self.edit_selection = char_start..char_start;
                    self.sync_after_edit(target, cx);
                    cx.notify();
                }
            }
        }
    }

    pub(super) fn save_edit_state(&mut self) {
        if let Some(target) = self.active_edit {
            let text = self.get_edit_text(target).to_string();
            self.edit_undo_stack.push((target, text, self.edit_selection.clone()));
            if self.edit_undo_stack.len() > 100 { self.edit_undo_stack.remove(0); }
            self.edit_redo_stack.clear();
        }
    }

    pub(super) fn edit_undo(&mut self, cx: &mut Context<Self>) {
        if let Some((target, text, selection)) = self.edit_undo_stack.pop() {
            let current_text = self.get_edit_text(target).to_string();
            self.edit_redo_stack.push((target, current_text, self.edit_selection.clone()));
            if let Some(field) = self.get_edit_text_mut(target) { *field = text; }
            self.edit_selection = selection;
            self.sync_after_edit(target, cx);
            cx.notify();
        }
    }

    pub(super) fn edit_redo(&mut self, cx: &mut Context<Self>) {
        if let Some((target, text, selection)) = self.edit_redo_stack.pop() {
            let current_text = self.get_edit_text(target).to_string();
            self.edit_undo_stack.push((target, current_text, self.edit_selection.clone()));
            if let Some(field) = self.get_edit_text_mut(target) { *field = text; }
            self.edit_selection = selection;
            self.sync_after_edit(target, cx);
            cx.notify();
        }
    }

    pub(super) fn edit_insert_text(&mut self, insert: &str, cx: &mut Context<Self>) {
        if let Some(target) = self.active_edit {
            self.save_edit_state();
            self.edit_delete_selection_no_save(cx);
            let char_pos = self.edit_selection.start;
            if let Some(text) = self.get_edit_text_mut(target) {
                let byte_pos = char_to_byte_offset(text, char_pos);
                text.insert_str(byte_pos, insert);
                let new_char_pos = char_pos + insert.chars().count();
                self.edit_selection = new_char_pos..new_char_pos;
                self.sync_after_edit(target, cx);
                cx.notify();
            }
            match target {
                EditTarget::ParamKey(i) | EditTarget::ParamValue(i) => {
                    let (key_empty, val_empty) = self.params.get(i)
                        .map_or((true, true), |p| (p.key.is_empty(), p.value.is_empty()));
                    if let Some(param) = self.params.get_mut(i) {
                        if !param.enabled && (!key_empty || !val_empty) {
                            param.enabled = true;
                            self.sync_url_from_params(cx);
                        }
                    }
                    if i + 1 == self.params.len()
                        && self.params.last().map_or(false, |p| !p.key.is_empty() || !p.value.is_empty())
                    {
                        self.params.push(KeyValuePair::default());
                        cx.notify();
                    }
                }
                EditTarget::HeaderKey(i) | EditTarget::HeaderValue(i) => {
                    let (key_empty, val_empty) = self.headers.get(i)
                        .map_or((true, true), |h| (h.key.is_empty(), h.value.is_empty()));
                    if let Some(header) = self.headers.get_mut(i) {
                        if !header.enabled && (!key_empty || !val_empty) { header.enabled = true; }
                    }
                    if i + 1 == self.headers.len()
                        && self.headers.last().map_or(false, |h| !h.key.is_empty() || !h.value.is_empty())
                    {
                        self.headers.push(KeyValuePair::default());
                        cx.notify();
                    }
                }
                _ => {}
            }
        }
    }

    pub(super) fn sync_after_edit(&mut self, target: EditTarget, cx: &mut Context<Self>) {
        match target {
            EditTarget::ParamKey(_) | EditTarget::ParamValue(_) => self.sync_url_from_params(cx),
            _ => {}
        }
    }

    pub(super) fn edit_index_for_x(&self, x: f32, char_width: f32) -> usize {
        if let Some(target) = self.active_edit {
            let char_count = self.get_edit_text(target).chars().count();
            if x <= 0.0 { 0 } else { ((x / char_width) as usize).min(char_count) }
        } else { 0 }
    }

    pub(super) fn handle_edit_mouse_down(&mut self, event: &MouseDownEvent, target: EditTarget, char_width: f32, cx: &mut Context<Self>) {
        self.edit_is_selecting = true;
        let text_start_x = self.edit_input_origins.get(&target).copied().unwrap_or(0.0);
        let click_x = (f32::from(event.position.x) - text_start_x).max(0.0);
        let index = self.edit_index_for_x(click_x, char_width);
        let effective_click = if event.click_count >= 4 { 1 } else { event.click_count };
        match effective_click {
            2 => {
                if let Some(target) = self.active_edit {
                    let text = self.get_edit_text(target);
                    self.edit_selection = find_word_start(&text, index)..find_word_end(&text, index);
                    cx.notify();
                }
            }
            3 => { self.edit_select_all(cx); }
            _ => {
                if event.modifiers.shift { self.edit_select_to(index, cx); }
                else { self.edit_move_to(index, cx); }
            }
        }
    }

    pub(super) fn handle_edit_mouse_move(&mut self, event: &MouseMoveEvent, char_width: f32, cx: &mut Context<Self>) {
        if self.edit_is_selecting {
            let text_start_x = self.active_edit
                .and_then(|t| self.edit_input_origins.get(&t).copied())
                .unwrap_or(0.0);
            let index = self.edit_index_for_x((f32::from(event.position.x) - text_start_x).max(0.0), char_width);
            if self.edit_selection.end != index { self.edit_selection.end = index; cx.notify(); }
        }
    }

    pub(super) fn handle_edit_mouse_up(&mut self, _event: &MouseUpEvent, _cx: &mut Context<Self>) {
        self.edit_is_selecting = false;
    }
}
