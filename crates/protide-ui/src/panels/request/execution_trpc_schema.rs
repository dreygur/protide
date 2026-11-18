use super::super::request_types::{PendingEditor, TrpcPlaygroundProc, TrpcProcKind};
use super::*;
use gpui::Context;

impl<E: WebSocketExecutor> RequestPanel<E> {
    pub(super) fn import_trpc_from_file(&mut self, cx: &mut Context<Self>) {
        file_dialog::prompt(
            cx,
            Pick::File,
            |d| {
                let d = d
                    .set_title("Import tRPC Schema")
                    .add_filter("JSON", &["json"]);
                match last_paths::last_dir("trpc_schema") {
                    Some(dir) => d.set_directory(dir),
                    None => d,
                }
            },
            |this, path, cx| {
                last_paths::save_last_dir("trpc_schema", &path);
                match std::fs::read_to_string(&path)
                    .map_err(|e| e.to_string())
                    .and_then(|s| protide_core::protocols::trpc::parse_trpc_schema(&s))
                {
                    Ok(procs) => {
                        for p in procs {
                            let proc = TrpcPlaygroundProc {
                                kind: if p.is_mutation {
                                    TrpcProcKind::Mutation
                                } else {
                                    TrpcProcKind::Query
                                },
                                name: p.name,
                            };
                            match this
                                .trpc_pg_procedures
                                .iter_mut()
                                .find(|existing| existing.name == proc.name)
                            {
                                Some(existing) => *existing = proc,
                                None => this.trpc_pg_procedures.push(proc),
                            }
                        }
                        this.trpc_pg_schema_error = None;
                    }
                    Err(e) => {
                        this.trpc_pg_schema_error = Some(e);
                    }
                }
                cx.notify();
            },
        );
    }

    pub(super) fn import_trpc_from_url(&mut self, cx: &mut Context<Self>) {
        let url = self
            .trpc_pg_import_url_input
            .read(cx)
            .value()
            .trim()
            .to_string();
        if url.is_empty() {
            return;
        }
        self.trpc_pg_schema_loading = true;
        self.trpc_pg_schema_error = None;
        cx.notify();

        cx.spawn(
            async move |this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
                let result = run_blocking(cx, move || {
                    let raw = protide_core::protocols::trpc::fetch_trpc_schema_raw(&url)?;
                    protide_core::protocols::trpc::parse_trpc_schema(&raw)
                })
                .await
                .unwrap_or_else(|_| Err("URL import thread panicked".to_string()));

                cx.update(|cx| {
                    let _ = this.update(cx, |panel, cx| {
                        panel.trpc_pg_schema_loading = false;
                        match result {
                            Ok(procs) => {
                                for p in procs {
                                    let proc = TrpcPlaygroundProc {
                                        kind: if p.is_mutation {
                                            TrpcProcKind::Mutation
                                        } else {
                                            TrpcProcKind::Query
                                        },
                                        name: p.name,
                                    };
                                    match panel
                                        .trpc_pg_procedures
                                        .iter_mut()
                                        .find(|existing| existing.name == proc.name)
                                    {
                                        Some(existing) => *existing = proc,
                                        None => panel.trpc_pg_procedures.push(proc),
                                    }
                                }
                                panel.trpc_pg_show_import_url = false;
                                panel.trpc_pg_schema_error = None;
                                panel.queue_editor(
                                    PendingEditor::TrpcPgImportUrlInput,
                                    String::new(),
                                );
                            }
                            Err(e) => {
                                panel.trpc_pg_schema_error = Some(e);
                            }
                        }
                        cx.notify();
                    });
                });
            },
        )
        .detach();
    }

    pub(super) fn fetch_trpc_schema(&mut self, cx: &mut Context<Self>) {
        self.trpc_pg_schema_loading = true;
        self.trpc_pg_schema_error = None;
        cx.notify();

        let url = self.url.clone();
        let env_state = self
            .explorer_panel
            .as_ref()
            .map(|p| p.read(cx).env_state().clone());
        let url = if let Some(ref env) = env_state {
            env.substitute(&url)
        } else {
            url
        };

        cx.spawn(
            async move |this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
                let result = run_blocking(cx, move || {
                    let raw = protide_core::protocols::trpc::fetch_trpc_schema_raw(&url);
                    raw.and_then(|json| protide_core::protocols::trpc::parse_trpc_schema(&json))
                })
                .await
                .unwrap_or_else(|_| Err("tRPC thread panicked".to_string()));

                cx.update(|cx| {
                    let _ = this.update(cx, |panel, cx| {
                        panel.trpc_pg_schema_loading = false;
                        match result {
                            Ok(procs) => {
                                panel.trpc_pg_procedures = procs
                                    .into_iter()
                                    .map(|p| TrpcPlaygroundProc {
                                        kind: if p.is_mutation {
                                            TrpcProcKind::Mutation
                                        } else {
                                            TrpcProcKind::Query
                                        },
                                        name: p.name,
                                    })
                                    .collect();
                                panel.trpc_pg_schema_error = None;
                            }
                            Err(e) => {
                                panel.trpc_pg_schema_error = Some(e);
                            }
                        }
                        cx.notify();
                    });
                });
            },
        )
        .detach();
    }
}
