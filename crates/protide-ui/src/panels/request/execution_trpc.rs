use gpui::Context;
use super::*;
use super::super::request_types::{PendingEditor, TrpcPlaygroundProc, TrpcProcKind};
use super::super::request_utils::status_text;

impl<E: WebSocketExecutor> RequestPanel<E> {
    pub(super) fn send_trpc_request(&mut self, cx: &mut Context<Self>) {
        if self.trpc_procedure.trim().is_empty() { return; }

        self.loading = true;
        cx.notify();

        let url = self.url.clone();
        let procedure = self.trpc_procedure.clone();
        let params = self.trpc_params_editor.read(cx).value().to_string();

        let env_state = self.explorer_panel.as_ref().map(|p| p.read(cx).env_state().clone());
        let substitute = |s: &str| -> String {
            if let Some(ref env) = env_state { env.substitute(s) } else { s.to_string() }
        };

        let url = substitute(&url);
        let procedure = substitute(&procedure);

        let mut headers: Vec<(String, String)> = self.headers.iter()
            .filter(|h| h.enabled && !h.key.is_empty())
            .map(|h| (substitute(&h.key), substitute(&h.value)))
            .collect();

        headers.extend(self.build_auth_headers(&substitute));

        let response_panel = self.response_panel.clone();
        log::info!("tRPC {} {}", url, procedure);

        cx.spawn(async move |this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
            let result = std::thread::spawn(move || {
                protide_core::protocols::trpc::execute_trpc(&url, &procedure, &params, headers)
            }).join().unwrap_or_else(|_| Err("tRPC thread panicked".to_string()));

            match result {
                Ok((body, elapsed, status_code)) => {
                    let body_size = body.len();
                    let _ = cx.update(|cx| {
                        response_panel.update(cx, |panel, cx| {
                            panel.set_response(ResponseData {
                                status: status_code,
                                status_text: status_text(status_code).to_string(),
                                headers: vec![("content-type".to_string(), "application/json".to_string())],
                                body,
                                time: elapsed,
                                size: body_size,
                            }, cx);
                        });
                    });
                }
                Err(e) => {
                    log::error!("tRPC error: {}", e);
                    let _ = cx.update(|cx| {
                        response_panel.update(cx, |panel, cx| {
                            let error_body = serde_json::json!({ "error": e }).to_string();
                            let error_size = error_body.len();
                            panel.set_response(ResponseData {
                                status: 500,
                                status_text: "tRPC Error".to_string(),
                                headers: vec![("content-type".to_string(), "application/json".to_string())],
                                body: error_body,
                                time: std::time::Duration::ZERO,
                                size: error_size,
                            }, cx);
                        });
                    });
                }
            }
            let _ = cx.update(|cx| { let _ = this.update(cx, |p, cx| { p.loading = false; cx.notify(); }); });
        }).detach();
    }

    pub(super) fn run_trpc_playground(&mut self, cx: &mut Context<Self>) {
        let idx = match self.trpc_pg_selected { Some(i) => i, None => return };
        let proc = match self.trpc_pg_procedures.get(idx) { Some(p) => p.clone(), None => return };

        self.trpc_pg_loading = true;
        self.trpc_pg_response = None;
        self.trpc_pg_error = None;
        self.trpc_pg_status = None;
        self.trpc_pg_elapsed = None;
        cx.notify();

        let url = self.url.clone();
        let procedure = proc.full_procedure();
        let params = self.trpc_params_editor.read(cx).value().to_string();

        let env_state = self.explorer_panel.as_ref().map(|p| p.read(cx).env_state().clone());
        let substitute = move |s: &str| -> String {
            if let Some(ref env) = env_state { env.substitute(s) } else { s.to_string() }
        };
        let url = substitute(&url);
        let procedure = substitute(&procedure);

        let mut headers: Vec<(String, String)> = self.headers.iter()
            .filter(|h| h.enabled && !h.key.is_empty())
            .map(|h| (substitute(&h.key), substitute(&h.value)))
            .collect();

        headers.extend(self.build_auth_headers(&substitute));

        log::info!("tRPC playground: {} {}", url, procedure);

        cx.spawn(async move |this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
            let result = std::thread::spawn(move || {
                protide_core::protocols::trpc::execute_trpc(&url, &procedure, &params, headers)
            }).join().unwrap_or_else(|_| Err("tRPC thread panicked".to_string()));

            let _ = cx.update(|cx| {
                let _ = this.update(cx, |panel, cx| {
                    panel.trpc_pg_loading = false;
                    match result {
                        Ok((body, elapsed, status)) => {
                            panel.trpc_pg_status = Some(status);
                            panel.trpc_pg_elapsed = Some(elapsed);
                            panel.trpc_pg_response = Some(body.clone());
                            panel.queue_editor(PendingEditor::TrpcPgResult, body);
                        }
                        Err(e) => {
                            panel.trpc_pg_status = Some(500);
                            panel.trpc_pg_error = Some(e.clone());
                            let val = serde_json::json!({ "error": e });
                            let body = serde_json::to_string_pretty(&val)
                                .unwrap_or_else(|_| e.clone());
                            panel.queue_editor(PendingEditor::TrpcPgResult, body);
                        }
                    }
                    cx.notify();
                });
            });
        }).detach();
    }

    pub(super) fn import_trpc_from_file(&mut self, cx: &mut Context<Self>) {
        let mut dialog = rfd::FileDialog::new()
            .set_title("Import tRPC Schema")
            .add_filter("JSON", &["json"]);
        if let Some(dir) = last_paths::last_dir("trpc_schema") {
            dialog = dialog.set_directory(dir);
        }
        let Some(path) = dialog.pick_file() else { return };
        last_paths::save_last_dir("trpc_schema", &path);
        match std::fs::read_to_string(&path)
            .map_err(|e| e.to_string())
            .and_then(|s| protide_core::protocols::trpc::parse_trpc_schema(&s))
        {
            Ok(procs) => {
                for p in procs {
                    self.trpc_pg_procedures.push(TrpcPlaygroundProc {
                        kind: if p.is_mutation { TrpcProcKind::Mutation } else { TrpcProcKind::Query },
                        name: p.name,
                    });
                }
                self.trpc_pg_schema_error = None;
            }
            Err(e) => { self.trpc_pg_schema_error = Some(e); }
        }
        cx.notify();
    }

    pub(super) fn import_trpc_from_url(&mut self, cx: &mut Context<Self>) {
        let url = self.trpc_pg_import_url_input.read(cx).value().trim().to_string();
        if url.is_empty() { return; }
        self.trpc_pg_schema_loading = true;
        self.trpc_pg_schema_error = None;
        cx.notify();

        cx.spawn(async move |this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
            let result = std::thread::spawn(move || {
                let raw = protide_core::protocols::trpc::fetch_trpc_schema_raw(&url)?;
                protide_core::protocols::trpc::parse_trpc_schema(&raw)
            }).join().unwrap_or_else(|_| Err("URL import thread panicked".to_string()));

            let _ = cx.update(|cx| {
                let _ = this.update(cx, |panel, cx| {
                    panel.trpc_pg_schema_loading = false;
                    match result {
                        Ok(procs) => {
                            for p in procs {
                                panel.trpc_pg_procedures.push(TrpcPlaygroundProc {
                                    kind: if p.is_mutation { TrpcProcKind::Mutation } else { TrpcProcKind::Query },
                                    name: p.name,
                                });
                            }
                            panel.trpc_pg_show_import_url = false;
                            panel.trpc_pg_schema_error = None;
                            panel.queue_editor(PendingEditor::TrpcPgImportUrlInput, String::new());
                        }
                        Err(e) => { panel.trpc_pg_schema_error = Some(e); }
                    }
                    cx.notify();
                });
            });
        }).detach();
    }

    pub(super) fn fetch_trpc_schema(&mut self, cx: &mut Context<Self>) {
        self.trpc_pg_schema_loading = true;
        self.trpc_pg_schema_error = None;
        cx.notify();

        let url = self.url.clone();
        let env_state = self.explorer_panel.as_ref().map(|p| p.read(cx).env_state().clone());
        let url = if let Some(ref env) = env_state { env.substitute(&url) } else { url };

        cx.spawn(async move |this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
            let result = std::thread::spawn(move || {
                let raw = protide_core::protocols::trpc::fetch_trpc_schema_raw(&url);
                raw.and_then(|json| protide_core::protocols::trpc::parse_trpc_schema(&json))
            }).join().unwrap_or_else(|_| Err("tRPC thread panicked".to_string()));

            let _ = cx.update(|cx| {
                let _ = this.update(cx, |panel, cx| {
                    panel.trpc_pg_schema_loading = false;
                    match result {
                        Ok(procs) => {
                            panel.trpc_pg_procedures = procs.into_iter().map(|p| TrpcPlaygroundProc {
                                kind: if p.is_mutation { TrpcProcKind::Mutation } else { TrpcProcKind::Query },
                                name: p.name,
                            }).collect();
                            panel.trpc_pg_schema_error = None;
                        }
                        Err(e) => { panel.trpc_pg_schema_error = Some(e); }
                    }
                    cx.notify();
                });
            });
        }).detach();
    }
}
