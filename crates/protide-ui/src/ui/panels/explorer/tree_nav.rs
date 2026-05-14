use gpui::Context;
use super::*;

impl ExplorerPanel {
    /// Toggle collections section expanded/collapsed
    pub(super) fn toggle_collections(&mut self, cx: &mut Context<Self>) {
        self.collections_expanded = !self.collections_expanded;
        cx.notify();
    }

    /// Toggle a folder's expanded state
    pub(super) fn toggle_collection_folder(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        fn toggle_in_items(items: &mut [CollectionItem], target: &PathBuf) -> bool {
            for item in items.iter_mut() {
                if item.path == *target && item.is_folder {
                    item.expanded = !item.expanded;
                    return true;
                }
                if item.is_folder && toggle_in_items(&mut item.children, target) {
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
                if item.is_folder {
                    item.expanded = false;
                    collapse(&mut item.children);
                }
            }
        }
        collapse(&mut self.collection_items);
        self.tree_scroll = 0.0;
        cx.notify();
    }

    pub(super) fn count_visible_items(items: &[CollectionItem]) -> usize {
        items.iter().map(|item| {
            1 + if item.is_folder && item.expanded {
                Self::count_visible_items(&item.children)
            } else {
                0
            }
        }).sum()
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
            if item.is_folder && item.expanded {
                result.extend(self.flatten_collection_items(&item.children, depth + 1));
            }
        }
        result
    }
}
