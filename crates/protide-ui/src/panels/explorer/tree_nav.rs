use super::*;
use gpui::Context;

impl ExplorerPanel {
    /// Load a request from a tree row, selecting it. `row_index` is the row's
    /// request index, `None` for a single request file.
    pub fn select_and_load(
        &mut self,
        path: PathBuf,
        row_index: Option<usize>,
        cx: &mut Context<Self>,
    ) {
        self.selected_item = Some(path.clone());
        self.selected_request = row_index;
        self.load_request_file(path, row_index.unwrap_or(0), cx);
    }

    /// Open a request from a tree row without silently dropping unsaved
    /// edits: switching away from a dirty request asks first, and reopening
    /// the dirty request itself keeps the edits.
    pub(super) fn open_request(
        &mut self,
        path: PathBuf,
        row_index: Option<usize>,
        cx: &mut Context<Self>,
    ) {
        let dirty = self
            .request_panel
            .as_ref()
            .and_then(|rp| rp.read(cx).dirty_location());
        match dirty {
            None => self.select_and_load(path, row_index, cx),
            Some(loc) if loc == (path.clone(), row_index.unwrap_or(0)) => {}
            Some(_) => {
                if let Some(win) = self.main_window.upgrade() {
                    win.update(cx, |win, cx| win.confirm_discard(path, row_index, cx));
                }
            }
        }
    }

    /// Toggle collections section expanded/collapsed
    pub(super) fn toggle_collections(&mut self, cx: &mut Context<Self>) {
        self.collections_expanded = !self.collections_expanded;
        cx.notify();
    }

    /// Toggle a folder's expanded state
    pub(super) fn toggle_collection_folder(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        fn toggle_in_items(items: &mut [CollectionItem], target: &PathBuf) -> bool {
            for item in items.iter_mut() {
                if item.path == *target && item.is_expandable() {
                    item.expanded = !item.expanded;
                    return true;
                }
                if item.is_expandable() && toggle_in_items(&mut item.children, target) {
                    return true;
                }
            }
            false
        }

        toggle_in_items(&mut self.collection_items, &path);
        cx.notify();
    }

    /// Recursively collapse all folders in the collection tree
    pub(super) fn collapse_all_folders(&mut self, cx: &mut Context<Self>) {
        fn collapse(items: &mut [CollectionItem]) {
            for item in items.iter_mut() {
                if item.is_expandable() {
                    item.expanded = false;
                    collapse(&mut item.children);
                }
            }
        }
        collapse(&mut self.collection_items);
        cx.notify();
    }

    /// Flatten collection items into a list with depth information for rendering
    pub(super) fn flatten_collection_items(
        &self,
        items: &[CollectionItem],
        depth: usize,
    ) -> Vec<(CollectionItem, usize)> {
        let mut result = Vec::new();
        for item in items {
            result.push((item.clone(), depth));
            if item.is_expandable() && item.expanded {
                result.extend(self.flatten_collection_items(&item.children, depth + 1));
            }
        }
        result
    }
}
