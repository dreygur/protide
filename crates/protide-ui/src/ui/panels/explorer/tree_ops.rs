use gpui::Context;
use super::*;

impl ExplorerPanel {
    /// Delete a collection item — shows confirm modal in MainWindow.
    pub(super) fn delete_collection_item(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let filename = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let message = if path.is_dir() {
            format!("Delete folder '{}' and all its contents?", filename)
        } else {
            format!("Delete '{}'?", filename)
        };
        if let Some(win) = self.main_window.upgrade() {
            win.update(cx, |win, cx| {
                win.show_confirm_delete(ModalState::confirm("Confirm Delete", message), path, cx);
            });
        }
        self.context_menu = None;
        cx.notify();
    }

    /// Called by MainWindow when user confirms deletion.
    pub fn execute_delete(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let result = if path.is_dir() {
            fs::remove_dir_all(&path)
        } else {
            fs::remove_file(&path)
        };
        match result {
            Ok(_) => {
                info!("Deleted: {}", path.display());
                self.refresh_collections(cx);
            }
            Err(e) => warn!("Delete failed {}: {}", path.display(), e),
        }
    }

    /// Start renaming a collection item
    pub(super) fn start_rename_item(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.rename_text = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        self.renaming_item = Some(path);
        self.context_menu = None;
        cx.notify();
    }

    /// Complete the rename operation
    pub(super) fn complete_rename(&mut self, cx: &mut Context<Self>) {
        if let Some(old_path) = self.renaming_item.take() {
            let new_name = self.rename_text.trim();
            if !new_name.is_empty() {
                let old_name = old_path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                if new_name != old_name
                    && let Some(parent) = old_path.parent() {
                        let new_path = parent.join(new_name);
                        match fs::rename(&old_path, &new_path) {
                            Ok(_) => {
                                info!("Renamed: {} → {}", old_name, new_name);
                                self.refresh_collections(cx);
                            }
                            Err(e) => error!("Rename failed {} → {}: {}", old_name, new_name, e),
                        }
                    }
            }
        }
        self.rename_text.clear();
        cx.notify();
    }

    /// Cancel the rename operation
    pub(super) fn cancel_rename(&mut self, cx: &mut Context<Self>) {
        self.renaming_item = None;
        self.rename_text.clear();
        cx.notify();
    }

    /// Close context menu
    pub(super) fn close_context_menu(&mut self, cx: &mut Context<Self>) {
        self.context_menu = None;
        cx.notify();
    }
}
