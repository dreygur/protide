use super::*;
use gpui::Context;

impl ExplorerPanel {
    pub fn create_new_request(&mut self, cx: &mut Context<Self>) {
        let default_dir = self.workspace_path.clone().or_else(dirs::home_dir);
        let start_dir = last_paths::last_dir("new_request")
            .or_else(|| last_paths::last_dir("save_request"))
            .or(default_dir);
        file_dialog::prompt(
            cx,
            Pick::Save,
            move |d| {
                let d = d
                    .set_title("Create New Request")
                    .set_file_name("new-request.http")
                    .add_filter("HTTP Request", &["http"]);
                match start_dir {
                    Some(dir) => d.set_directory(dir),
                    None => d,
                }
            },
            |this, path, cx| {
                last_paths::save_last_dir("new_request", &path);
                last_paths::save_last_dir("save_request", &path);
                let path = if path.extension().is_none() || path.extension().unwrap() != "http" {
                    path.with_extension("http")
                } else {
                    path
                };
                let content =
                    "### New Request\n# @name new-request\n\nGET https://api.example.com\n";
                match fs::write(&path, content) {
                    Ok(_) => {
                        info!("Created: {}", path.display());
                        if let Some(workspace) = &this.workspace_path
                            && path.starts_with(workspace)
                        {
                            this.collection_items = this.scan_directory(workspace);
                        }
                        this.open_request(path, None, cx);
                    }
                    Err(e) => error!("Failed to create request {}: {}", path.display(), e),
                }
            },
        );
    }

    pub(super) fn clone_file(&mut self, source_path: PathBuf, cx: &mut Context<Self>) {
        let content = match fs::read_to_string(&source_path) {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to read {}: {}", source_path.display(), e);
                return;
            }
        };
        let parent = match source_path.parent() {
            Some(p) => p.to_path_buf(),
            None => return,
        };
        let stem = source_path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "request".to_string());
        let mut new_path = parent.join(format!("{}-copy.http", stem));
        let mut n = 2usize;
        while new_path.exists() {
            new_path = parent.join(format!("{}-copy-{}.http", stem, n));
            n += 1;
        }
        match fs::write(&new_path, &content) {
            Ok(_) => {
                info!("Cloned: {}", new_path.display());
                if let Some(workspace) = &self.workspace_path
                    && new_path.starts_with(workspace)
                {
                    self.collection_items = self.scan_directory(workspace);
                }
                self.open_request(new_path, None, cx);
            }
            Err(e) => {
                error!("Failed to clone: {}", e);
                cx.notify();
            }
        }
    }

    pub(super) fn create_new_file_in_folder(
        &mut self,
        folder_path: PathBuf,
        cx: &mut Context<Self>,
    ) {
        file_dialog::prompt(
            cx,
            Pick::Save,
            move |d| {
                d.set_title("Create New Request")
                    .set_file_name("new-request.http")
                    .set_directory(&folder_path)
                    .add_filter("HTTP Request", &["http"])
            },
            |this, path, cx| {
                last_paths::save_last_dir("new_request", &path);
                last_paths::save_last_dir("save_request", &path);
                let path = if path.extension().is_none_or(|e| e != "http") {
                    path.with_extension("http")
                } else {
                    path
                };
                let content =
                    "### New Request\n# @name new-request\n\nGET https://api.example.com\n";
                match fs::write(&path, content) {
                    Ok(_) => {
                        info!("Created: {}", path.display());
                        if let Some(workspace) = &this.workspace_path
                            && path.starts_with(workspace)
                        {
                            this.collection_items = this.scan_directory(workspace);
                        }
                        this.open_request(path, None, cx);
                    }
                    Err(e) => error!("Failed to create request: {}", e),
                }
            },
        );
    }
}
