//! Form body rendering for RequestPanel


use gpui::{
    div, prelude::*, px, Context, IntoElement,
    ParentElement, SharedString, Styled,
};

use crate::theme;
use protide_core::execution::ws::WebSocketExecutor;
use super::super::request_types::{BodyType, EditTarget, FormFieldType};
use super::RequestPanel;

impl<E: WebSocketExecutor> RequestPanel<E> {
    pub(super) fn render_form_body(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let form_len = self.form_data.len();
        let active_edit = self.active_edit;
        let edit_selection = self.edit_selection.clone();
        let enabled_count = self.form_data.iter().filter(|f| f.enabled && !f.key.is_empty()).count();

        let form_data: Vec<_> = self.form_data.iter().enumerate().map(|(i, field)| {
            (i, field.enabled, field.key.clone(), field.value.clone(), field.field_type.clone(), field.file_path.is_some())
        }).collect();

        let mut container = div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(2.0))
            .track_focus(&self.edit_focus);

        // Toolbar row with body type buttons
        container = container.child(
            div()
                .h(px(40.0))
                .w_full()
                .flex()
                .items_center()
                .border_b_1()
                .border_color(theme.colors.border)
                .bg(theme.colors.bg_primary)
                .child(self.render_body_type_btn_form("body-type-json-form", "JSON", BodyType::Json, cx))
                .child(self.render_body_type_btn_form("body-type-raw-form", "Raw", BodyType::Raw, cx))
                .child(self.render_body_type_btn_form("body-type-xml-form", "XML", BodyType::Xml, cx))
                .child(self.render_body_type_btn_form("body-type-form-form", "Form", BodyType::Form, cx))
                .child(self.render_body_type_btn_form("body-type-binary-form", "Binary", BodyType::Binary, cx))
                .child(div().flex_1())
                .child(
                    div()
                        .px(px(12.0))
                        .py(px(2.0))
                        .bg(theme.colors.accent.opacity(0.12))
                        .text_size(px(10.0))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(theme.colors.accent)
                        .child(format!("{} fields", enabled_count))
                )
        );

        // Table header with extra TYPE column
        container = container.child(
            div()
                .w_full().flex().items_center()
                .gap(px(8.0))
                .py(px(6.0))
                .border_b_1().border_color(theme.colors.border)
                .mb(px(4.0))
                .child(div().w(px(12.0)))
                .child(div().size(px(16.0)))
                .child(
                    div()
                        .w(px(130.0))
                        .text_size(px(10.0))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.colors.accent.opacity(0.7))
                        .child("KEY")
                )
                .child(
                    div()
                        .w(px(50.0))
                        .text_size(px(10.0))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.colors.text_secondary)
                        .child("TYPE")
                )
                .child(
                    div()
                        .flex_1().flex().items_center().justify_between()
                        .child(
                            div()
                                .text_size(px(10.0))
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(theme.colors.text_secondary)
                                .child("VALUE")
                        )
                        .child(self.render_count_badge(enabled_count, "active", cx))
                )
                .child(div().size(px(28.0)))
        );

        // Form fields list
        let dragging_form = self.form_row_drag.is_some();
        for (i, is_enabled, key, value, field_type, has_file) in form_data {
            let can_remove = form_len > 1;
            let is_file_type = field_type == FormFieldType::File;
            let is_editing_key = active_edit == Some(EditTarget::FormKey(i));
            let is_editing_value = active_edit == Some(EditTarget::FormValue(i));
            let is_row_editing = is_editing_key || is_editing_value;
            let drop_here = dragging_form && self.form_row_drag_over == Some(i);

            container = container.child(
                div()
                    .id(SharedString::from(format!("form-row-{}", i)))
                    .w_full()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .py(px(4.0))
                    .px(px(2.0))
                    .when(!is_row_editing, |el| el.hover(|s| s.bg(theme.colors.bg_tertiary.opacity(0.3))))
                    .when(drop_here, |el| el.border_t_2().border_color(theme.colors.accent))
                    .child(self.render_form_row_drag_handle(i, cx))
                    .child(
                        self.render_kv_checkbox(format!("form-checkbox-{}", i).into(), is_enabled, cx)
                            .on_click({
                                let idx = i;
                                cx.listener(move |this, _, _, cx| {
                                    this.toggle_form_field(idx, cx);
                                })
                            })
                    )
                    .child(
                        self.render_kv_input(
                            format!("form-key-{}", i),
                            EditTarget::FormKey(i),
                            &key,
                            "key",
                            is_editing_key,
                            edit_selection.clone(),
                            px(130.0),
                            cx,
                        )
                    )
                    // Type selector
                    .child(
                        div()
                            .id(SharedString::from(format!("form-type-{}", i)))
                            .w(px(50.0))
                            .h(px(24.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .border_1()
                            .border_color(theme.colors.border)
                            .bg(theme.colors.bg_tertiary)
                            .hover(|s| s.border_color(theme.colors.text_muted))
                            .text_size(px(10.0))
                            .text_color(if is_file_type { theme.colors.accent } else { theme.colors.text_muted })
                            .on_click({
                                let idx = i;
                                cx.listener(move |this, _, _, cx| {
                                    this.toggle_form_field_type(idx, cx);
                                })
                            })
                            .child(if is_file_type { "File" } else { "Text" })
                    )
                    .when(!is_file_type, |el| {
                        el.child(
                            self.render_kv_input_flex(
                                format!("form-value-{}", i),
                                EditTarget::FormValue(i),
                                &value,
                                "value",
                                is_editing_value,
                                edit_selection.clone(),
                                cx,
                            )
                        )
                    })
                    .when(is_file_type, |el| {
                        el.child(
                            div()
                                .id(SharedString::from(format!("form-file-{}", i)))
                                .flex_1()
                                .h(px(24.0))
                                .flex()
                                .items_center()
                                .gap(px(8.0))
                                .child(
                                    div()
                                        .id(SharedString::from(format!("form-file-btn-{}", i)))
                                        .px(px(10.0))
                                        .h(px(24.0))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .cursor_pointer()
                                        .bg(theme.colors.bg_tertiary)
                                        .border_1()
                                        .border_color(theme.colors.border)
                                        .hover(|s| s.border_color(theme.colors.accent))
                                        .text_size(px(11.0))
                                        .text_color(theme.colors.text_secondary)
                                        .on_click({
                                            let idx = i;
                                            cx.listener(move |this, _, _, cx| {
                                                this.select_form_file(idx, cx);
                                            })
                                        })
                                        .child("Choose File")
                                )
                                .when(has_file, |el| {
                                    el.child(
                                        div()
                                            .flex_1()
                                            .min_w(px(0.0))
                                            .overflow_hidden()
                                            .text_size(px(11.0))
                                            .text_color(theme.colors.text_primary)
                                            .child(value.clone())
                                    )
                                })
                                .when(!has_file, |el| {
                                    el.child(
                                        div()
                                            .text_size(px(11.0))
                                            .text_color(theme.colors.text_muted)
                                            .child("No file selected")
                                    )
                                })
                        )
                    })
                    .child(
                        self.render_kv_remove_btn(format!("form-remove-{}", i).into(), can_remove, cx)
                            .when(can_remove, |el| el.on_click({
                                let idx = i;
                                cx.listener(move |this, _, _, cx| {
                                    this.remove_form_field(idx, cx);
                                })
                            }))
                    )
            );
        }

        container = container.child(
            div().w_full().pt(px(8.0)).child(
                self.render_kv_add_btn("add-form-field-btn", "+ Add Field", cx)
                    .on_click(cx.listener(|this, _, _, cx| { this.add_form_field(cx); }))
            )
        );

        container.into_any_element()
    }

    fn render_body_type_btn_form(&self, id: &'static str, label: &'static str, bt: BodyType, cx: &Context<Self>) -> impl IntoElement {
        let theme = theme::current(cx);
        let is_active = self.body_type == bt;
        div()
            .id(id)
            .h_full()
            .px(px(16.0))
            .flex()
            .items_center()
            .cursor_pointer()
            .border_b_2()
            .when(is_active, |el| el.border_color(theme.colors.accent))
            .when(!is_active, |el| el.border_color(gpui::transparent_black()).hover(|s| s.bg(theme.colors.hover_overlay)))
            .on_click(cx.listener(move |this, _, _, cx| { this.set_body_type(bt, cx); }))
            .child(div().text_size(px(13.0))
                .font_weight(if is_active { gpui::FontWeight::MEDIUM } else { gpui::FontWeight::NORMAL })
                .text_color(if is_active { theme.colors.text_primary } else { theme.colors.text_secondary })
                .child(label))
    }
}
