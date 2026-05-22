use std::collections::HashMap;
use gpui::{Context, IntoElement, MouseButton, ParentElement, Styled, div, px, prelude::*};
use protide_core::execution::ws::WebSocketExecutor;
use crate::theme;
use crate::panels::request_types::DataRunRow;
use super::RequestPanel;

impl<E: WebSocketExecutor> RequestPanel<E> {
    pub(super) fn render_data_tab(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = theme::current(cx);
        let csv_path = self.csv_path.clone();
        let results = self.data_results.clone();
        let running = self.data_running;

        let csv_label = csv_path
            .as_ref()
            .and_then(|p| p.file_name()?.to_str().map(str::to_string))
            .unwrap_or_else(|| "No CSV selected".to_string());

        div()
            .w_full()
            .flex()
            .flex_col()
            .gap_3()
            // CSV file picker
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .id("data-csv-btn")
                            .px_2()
                            .h(px(26.0))
                            .flex()
                            .items_center()
                            .rounded_sm()
                            .bg(theme.colors.bg_tertiary)
                            .text_xs()
                            .text_color(theme.colors.text_primary)
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.colors.border))
                            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                                this.pick_csv_file(cx);
                            }))
                            .child("Browse CSV"),
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_xs()
                            .text_color(theme.colors.text_muted)
                            .child(csv_label),
                    ),
            )
            // Run button
            .child(
                div()
                    .id("data-run-btn")
                    .w_full()
                    .h(px(28.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_sm()
                    .bg(if running {
                        theme.colors.bg_tertiary
                    } else {
                        theme.colors.accent
                    })
                    .text_color(if running {
                        theme.colors.text_muted
                    } else {
                        gpui::white()
                    })
                    .text_xs()
                    .cursor_pointer()
                    .hover(|s| s.opacity(0.85))
                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| {
                        if !this.data_running {
                            this.run_with_csv(cx);
                        }
                    }))
                    .child(if running { "Running…" } else { "Run with CSV" }),
            )
            // Results table
            .when(!results.is_empty(), |el| {
                el.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_px()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .h(px(22.0))
                                .px_1()
                                .bg(theme.colors.bg_tertiary)
                                .child(div().w(px(40.0)).text_xs().text_color(theme.colors.text_muted).child("Row"))
                                .child(div().w(px(60.0)).text_xs().text_color(theme.colors.text_muted).child("Status"))
                                .child(div().flex_1().text_xs().text_color(theme.colors.text_muted).child("Result")),
                        )
                        .children(results.iter().map(|row| {
                            let status_txt = row.status
                                .map(|s| s.to_string())
                                .unwrap_or_else(|| "-".to_string());
                            let result_txt = if row.passed {
                                "Passed".to_string()
                            } else {
                                row.error.clone().unwrap_or_else(|| "Failed".to_string())
                            };
                            let result_color = if row.passed {
                                theme.colors.status_success
                            } else {
                                theme.colors.status_client_error
                            };
                            let row_num = row.row + 1;

                            div()
                                .flex()
                                .items_center()
                                .h(px(22.0))
                                .px_1()
                                .child(div().w(px(40.0)).text_xs().text_color(theme.colors.text_muted).child(format!("{row_num}")))
                                .child(div().w(px(60.0)).text_xs().text_color(theme.colors.text_secondary).child(status_txt))
                                .child(div().flex_1().text_xs().text_color(result_color).child(result_txt))
                        })),
                )
            })
            .into_any_element()
    }

    pub(super) fn pick_csv_file(&mut self, cx: &mut Context<Self>) {
        let dialog = rfd::FileDialog::new()
            .set_title("Select CSV File")
            .add_filter("CSV", &["csv"]);
        if let Some(path) = dialog.pick_file() {
            self.csv_path = Some(path);
            self.data_results.clear();
            cx.notify();
        }
    }

    pub(super) fn run_with_csv(&mut self, cx: &mut Context<Self>) {
        let Some(csv_path) = self.csv_path.clone() else { return; };
        let csv_content = match std::fs::read_to_string(&csv_path) {
            Ok(s) => s,
            Err(e) => {
                log::error!("Failed to read CSV: {e}");
                return;
            }
        };

        let env_state = self.explorer_panel.as_ref()
            .map(|p| p.read(cx).env_state().clone());
        let env_vars: HashMap<String, String> = env_state.as_ref()
            .and_then(|e| e.active())
            .map(|env| env.variables.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default();

        let request = self.build_http_request_model(cx);

        self.data_results.clear();
        self.data_running = true;
        cx.notify();

        let (tx, rx) = async_channel::unbounded::<protide_core::collection_runner::data_driven::DataDrivenProgress>();

        std::thread::spawn(move || {
            let _ = protide_core::collection_runner::data_driven::run_data_driven(
                &request, &csv_content, env_vars, tx,
            );
        });

        cx.spawn(async move |panel, cx| {
            while let Ok(event) = rx.recv().await {
                use protide_core::collection_runner::data_driven::DataDrivenProgress;
                let done = matches!(event, DataDrivenProgress::Done);
                panel.update(cx, |this, cx| {
                    match event {
                        DataDrivenProgress::Starting { .. } => {}
                        DataDrivenProgress::Completed { result, .. } => {
                            let row = DataRunRow {
                                row: result.row,
                                status: result.result.as_ref().ok().map(|r| r.status),
                                passed: result.result.is_ok(),
                                error: result.result.err(),
                            };
                            this.data_results.push(row);
                            cx.notify();
                        }
                        DataDrivenProgress::Done => {
                            this.data_running = false;
                            cx.notify();
                        }
                    }
                }).ok();
                if done { break; }
            }
        }).detach();
    }

    pub(super) fn build_http_request_model(&self, cx: &gpui::App) -> http_parser::Request {
        let body_content = self.body_editor.read(cx).value().to_string();
        let pre = self.pre_script_editor.read(cx).value().to_string();
        let post = self.post_script_editor.read(cx).value().to_string();
        let tests = self.tests_editor.read(cx).value().to_string();

        let headers: Vec<http_parser::KeyValue> = self.headers.iter()
            .filter(|h| !h.key.is_empty())
            .map(|h| http_parser::KeyValue { key: h.key.clone(), value: h.value.clone(), enabled: h.enabled })
            .collect();

        http_parser::Request {
            meta: http_parser::RequestMeta::default(),
            method: http_parser::HttpMethod::from_str(self.method.as_str())
                .unwrap_or(http_parser::HttpMethod::Get),
            url: self.url.clone(),
            headers,
            body: if body_content.is_empty() { None } else { Some(body_content) },
            scripts: http_parser::Scripts {
                pre_script: if pre.is_empty() { None } else { Some(pre) },
                post_script: if post.is_empty() { None } else { Some(post) },
                tests: if tests.is_empty() { None } else { Some(tests) },
            },
            line: 0,
        }
    }
}
