use gpui::Entity;
use std::path::PathBuf;
use crate::panels::explorer::{CollectionItem, ExplorerPanel};

pub mod render;

pub struct DocsPanel {
    pub(super) explorer: Option<Entity<ExplorerPanel>>,
    pub(super) selected_path: Option<PathBuf>,
}

impl DocsPanel {
    pub fn new() -> Self {
        Self { explorer: None, selected_path: None }
    }

    pub fn set_explorer(&mut self, explorer: Entity<ExplorerPanel>) {
        self.explorer = Some(explorer);
    }

    pub(super) fn flatten(items: &[CollectionItem], depth: usize) -> Vec<(CollectionItem, usize)> {
        let mut out = Vec::new();
        for item in items {
            out.push((item.clone(), depth));
            if item.is_folder {
                out.extend(Self::flatten(&item.children, depth + 1));
            }
        }
        out
    }
}
