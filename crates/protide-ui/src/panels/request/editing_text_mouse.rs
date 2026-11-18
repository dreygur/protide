use super::*;
use crate::components::{find_word_end, find_word_start};
use gpui::{Context, MouseDownEvent, MouseMoveEvent, MouseUpEvent};

impl<E: WebSocketExecutor> RequestPanel<E> {
    pub(super) fn edit_index_for_x(&self, x: f32, char_width: f32) -> usize {
        if let Some(target) = self.active_edit {
            let char_count = self.get_edit_text(target).chars().count();
            if x <= 0.0 {
                0
            } else {
                ((x / char_width) as usize).min(char_count)
            }
        } else {
            0
        }
    }

    pub(super) fn handle_edit_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        target: EditTarget,
        char_width: f32,
        cx: &mut Context<Self>,
    ) {
        self.edit_is_selecting = true;
        let text_start_x = self.edit_input_origins.get(&target).copied().unwrap_or(0.0);
        let click_x = (f32::from(event.position.x) - text_start_x).max(0.0);
        let index = self.edit_index_for_x(click_x, char_width);
        let effective_click = if event.click_count >= 4 {
            1
        } else {
            event.click_count
        };
        match effective_click {
            2 => {
                if let Some(target) = self.active_edit {
                    let text = self.get_edit_text(target);
                    self.edit_selection = find_word_start(text, index)..find_word_end(text, index);
                    cx.notify();
                }
            }
            3 => {
                self.edit_select_all(cx);
            }
            _ => {
                if event.modifiers.shift {
                    self.edit_select_to(index, cx);
                } else {
                    self.edit_move_to(index, cx);
                }
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
            let index = self.edit_index_for_x(
                (f32::from(event.position.x) - text_start_x).max(0.0),
                char_width,
            );
            if self.edit_selection.end != index {
                self.edit_selection.end = index;
                cx.notify();
            }
        }
    }

    pub(super) fn handle_edit_mouse_up(&mut self, _event: &MouseUpEvent, _cx: &mut Context<Self>) {
        self.edit_is_selecting = false;
    }
}
