use gpui::Context;
use super::*;
use super::util::{sanitize_filename, request_to_http_content};

impl ExplorerPanel {
    pub fn import_collection(&mut self, cx: &mut Context<Self>) {
        let mut dialog = rfd::FileDialog::new()
            .set_title("Import Collection")
            .add_filter("All Supported", &["json", "yaml", "yml", "bru", "txt", "curl"])
            .add_filter("Postman Collection", &["json"])
            .add_filter("OpenAPI/Swagger", &["json", "yaml", "yml"])
            .add_filter("Bruno Collection", &["bru"])
            .add_filter("cURL Command", &["txt", "curl"]);
        if let Some(dir) = last_paths::last_dir("import_collection") {
            dialog = dialog.set_directory(dir);
        }
        if let Some(file_path) = dialog.pick_file() {
            last_paths::save_last_dir("import_collection", &file_path);
            if let Ok(content) = fs::read_to_string(&file_path) {
                match protide_core::import::import(&content) {
                    Ok(result) => {
                        let output_dir = self.workspace_path.clone()
                            .or_else(|| file_path.parent().map(|p| p.to_path_buf()))
                            .unwrap_or_else(|| PathBuf::from("."));
                        let collection_dir = if let Some(name) = &result.name {
                            let dir = output_dir.join(sanitize_filename(name));
                            let _ = fs::create_dir_all(&dir);
                            dir
                        } else {
                            output_dir.clone()
                        };
                        let mut created = 0;
                        for (i, request) in result.requests.iter().enumerate() {
                            let filename = request.meta.name.as_ref()
                                .map(|n| sanitize_filename(n))
                                .unwrap_or_else(|| format!("request-{}", created + 1));
                            let parent = if let Some(Some(folder)) = result.request_folders.get(i) {
                                let sub = collection_dir.join(sanitize_filename(folder));
                                let _ = fs::create_dir_all(&sub);
                                sub
                            } else {
                                collection_dir.clone()
                            };
                            let filepath = parent.join(format!("{}.http", filename));
                            match request_to_http_content(request) {
                                Ok(c) => match fs::write(&filepath, &c) {
                                    Ok(_) => created += 1,
                                    Err(e) => warn!("Failed to write {}: {}", filepath.display(), e),
                                },
                                Err(e) => warn!("Failed to convert '{}': {}", filename, e),
                            }
                        }
                        if self.workspace_path.is_some() {
                            self.collection_items = self.scan_directory(&output_dir);
                        } else {
                            self.load_collection_from_path(collection_dir, cx);
                        }
                        info!("Imported {} request(s)", created);
                        let mut msg = format!("Imported {} request(s).", created);
                        if !result.warnings.is_empty() {
                            msg.push_str(&format!("\n\n{} warning(s):\n{}", result.warnings.len(), result.warnings.join("\n")));
                        }
                        if let Some(win) = self.main_window.upgrade() {
                            win.update(cx, |win, cx| win.show_modal("Import Complete", &msg, cx));
                        }
                    }
                    Err(e) => {
                        error!("Import failed: {}", e);
                        if let Some(win) = self.main_window.upgrade() {
                            win.update(cx, |win, cx| win.show_modal("Import Failed", e, cx));
                        }
                    }
                }
            }
        }
        cx.notify();
    }

    pub(super) fn run_collection_folder(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let env_vars: std::collections::HashMap<String, String> = self.env_state
            .active()
            .map(|e| e.variables.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default();
        if let Some(win) = self.main_window.upgrade() {
            self.context_menu = None;
            cx.notify();
            win.update(cx, |win, cx| win.open_runner(path, env_vars, cx));
        }
    }

    pub(super) fn export_openapi_folder(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let dialog = rfd::FileDialog::new()
            .set_title("Export OpenAPI 3.0")
            .add_filter("JSON", &["json"])
            .set_file_name("openapi.json");
        self.context_menu = None;
        cx.notify();
        if let Some(save_path) = dialog.save_file() {
            let (title, msg) = match protide_core::export::export_openapi(&path) {
                Ok(content) => match std::fs::write(&save_path, &content) {
                    Ok(_) => {
                        info!("Exported OpenAPI to: {}", save_path.display());
                        ("Export Complete", format!("OpenAPI spec saved to {}", save_path.display()))
                    }
                    Err(e) => ("Export Failed", format!("Failed to write file: {}", e)),
                },
                Err(e) => ("Export Failed", e),
            };
            if let Some(win) = self.main_window.upgrade() {
                win.update(cx, |win, cx| win.show_modal(title, msg, cx));
            }
        }
    }

    pub fn export_docs(&mut self, cx: &mut Context<Self>) {
        let Some(workspace) = &self.workspace_path else { return; };
        let dialog = rfd::FileDialog::new()
            .set_title("Export API Documentation")
            .add_filter("Markdown", &["md"])
            .add_filter("HTML", &["html"])
            .set_file_name("api-docs.md");
        if let Some(save_path) = dialog.save_file() {
            let is_html = save_path.extension().and_then(|e| e.to_str()) == Some("html");
            let format = if is_html { protide_core::export::ExportFormat::Html } else { protide_core::export::ExportFormat::Markdown };
            let (title, msg) = match protide_core::export::export_collection(workspace, format) {
                Ok(content) => match std::fs::write(&save_path, &content) {
                    Ok(_) => {
                        info!("Exported docs to: {}", save_path.display());
                        ("Export Complete", format!("Documentation saved to {}", save_path.display()))
                    }
                    Err(e) => ("Export Failed", format!("Failed to write file: {}", e)),
                },
                Err(e) => ("Export Failed", e),
            };
            if let Some(win) = self.main_window.upgrade() {
                win.update(cx, |win, cx| win.show_modal(title, msg, cx));
            }
        }
    }
}
