//! Params and Headers tab rendering for RequestPanel


use gpui::{
    div, prelude::*, px, Context, IntoElement,
    ParentElement, SharedString, Styled,
};

use crate::theme;
use protide_core::execution::ws::WebSocketExecutor;
use super::super::request_types::{EditTarget};
use super::RequestPanel;

impl<E: WebSocketExecutor> RequestPanel<E> {
    pub(super) fn render_params_tab(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let active_edit = self.active_edit;
        let edit_selection = self.edit_selection.clone();
        let enabled_count = self.params.iter().filter(|p| p.enabled && !p.key.is_empty()).count();

        let params_data: Vec<_> = self.params.iter().enumerate().map(|(i, param)| {
            (i, param.enabled, param.key.clone(), param.value.clone())
        }).collect();

        let mut container = div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(2.0))
            .track_focus(&self.edit_focus);

        container = container.child(self.render_kv_table_header("KEY", "VALUE", enabled_count, cx));

        for (i, is_enabled, key, value) in params_data {
            let can_remove = !key.is_empty() || !value.is_empty();
            let is_editing_key = active_edit == Some(EditTarget::ParamKey(i));
            let is_editing_value = active_edit == Some(EditTarget::ParamValue(i));
            let is_row_editing = is_editing_key || is_editing_value;

            container = container.child(
                div()
                    .id(SharedString::from(format!("param-row-{}", i)))
                    .w_full()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .py(px(4.0))
                    .px(px(2.0))
                    .when(!is_row_editing, |el| el.hover(|s| s.bg(theme.colors.bg_tertiary.opacity(0.3))))
                    .child(
                        self.render_kv_checkbox(format!("param-checkbox-{}", i).into(), is_enabled, cx)
                            .on_click(cx.listener(move |this, _, _, cx| this.toggle_param(i, cx)))
                    )
                    .child(
                        self.render_kv_input(
                            format!("param-key-{}", i),
                            EditTarget::ParamKey(i),
                            &key,
                            "Key",
                            is_editing_key,
                            if is_editing_key { edit_selection.clone() } else { 0..0 },
                            px(self.kv_col_key_w),
                            cx,
                        )
                    )
                    .child(div().w(px(4.0)))
                    .child(
                        self.render_kv_input_flex(
                            format!("param-value-{}", i),
                            EditTarget::ParamValue(i),
                            &value,
                            "Value",
                            is_editing_value,
                            if is_editing_value { edit_selection.clone() } else { 0..0 },
                            cx,
                        )
                    )
                    .child(
                        self.render_kv_remove_btn(format!("param-remove-{}", i).into(), can_remove, cx)
                            .when(can_remove, |el| el.on_click(cx.listener(move |this, _, _, cx| this.remove_param(i, cx))))
                    )
            );
        }

        container.into_any_element()
    }

    pub(super) fn render_headers_tab(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let active_edit = self.active_edit;
        let edit_selection = self.edit_selection.clone();
        let enabled_count = self.headers.iter().filter(|h| h.enabled && !h.key.is_empty()).count();

        let headers_data: Vec<_> = self.headers.iter().enumerate().map(|(i, header)| {
            (i, header.enabled, header.key.clone(), header.value.clone())
        }).collect();

        let mut container = div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(2.0))
            .track_focus(&self.edit_focus);

        container = container.child(self.render_kv_table_header("HEADER", "VALUE", enabled_count, cx));

        for (i, is_enabled, key, value) in headers_data {
            let can_remove = !key.is_empty() || !value.is_empty();
            let is_editing_key = active_edit == Some(EditTarget::HeaderKey(i));
            let is_editing_value = active_edit == Some(EditTarget::HeaderValue(i));
            let is_row_editing = is_editing_key || is_editing_value;

            container = container.child(
                div()
                    .id(SharedString::from(format!("header-row-{}", i)))
                    .w_full()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .py(px(4.0))
                    .px(px(2.0))
                    .when(!is_row_editing, |el| el.hover(|s| s.bg(theme.colors.bg_tertiary.opacity(0.3))))
                    .child(
                        self.render_kv_checkbox(format!("header-checkbox-{}", i).into(), is_enabled, cx)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.toggle_header(i, cx);
                            }))
                    )
                    .child(
                        self.render_kv_input(
                            format!("header-key-{}", i),
                            EditTarget::HeaderKey(i),
                            &key,
                            "Header name",
                            is_editing_key,
                            if is_editing_key { edit_selection.clone() } else { 0..0 },
                            px(self.kv_col_key_w),
                            cx,
                        )
                    )
                    .child(div().w(px(4.0)))
                    .child(
                        self.render_kv_input_flex(
                            format!("header-value-{}", i),
                            EditTarget::HeaderValue(i),
                            &value,
                            "Value",
                            is_editing_value,
                            if is_editing_value { edit_selection.clone() } else { 0..0 },
                            cx,
                        )
                    )
                    .child(
                        self.render_kv_remove_btn(format!("header-remove-{}", i).into(), can_remove, cx)
                            .when(can_remove, |el| el.on_click(cx.listener(move |this, _, _, cx| {
                                this.remove_header(i, cx);
                            })))
                    )
            );
        }

        container.into_any_element()
    }
}
