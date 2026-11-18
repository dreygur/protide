use super::*;
use gpui::{Context, MouseDownEvent, MouseMoveEvent, MouseUpEvent};

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

        let effective_click = if event.click_count >= 4 {
            1
        } else {
            event.click_count
        };

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

    pub(super) fn handle_rename_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        cx: &mut Context<Self>,
    ) {
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
            _ => {
                self.rename_selection = idx..idx;
            }
        }
        self.rename_is_selecting = true;
        cx.notify();
    }

    pub(super) fn handle_rename_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        cx: &mut Context<Self>,
    ) {
        if self.rename_is_selecting {
            let click_x = (f32::from(event.position.x) - self.rename_input_origin).max(0.0);
            let idx = ((click_x / 7.2) as usize).min(self.rename_text.chars().count());
            if self.rename_selection.end != idx {
                self.rename_selection.end = idx;
                cx.notify();
            }
        }
    }
}
