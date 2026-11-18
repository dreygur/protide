use super::*;
use log::debug;

fn request_badge(req: &http_parser::Request) -> String {
    match req.protocol() {
        Protocol::GraphQL => "GQL",
        Protocol::WebSocket => "WS",
        Protocol::Grpc => "GRPC",
        Protocol::SocketIO => "SIO",
        Protocol::Trpc => "TRPC",
        Protocol::Http => req.method.as_str(),
    }
    .to_string()
}

/// Tree row for a .http file. A file holding several requests lists them as
/// children so each one is reachable; a single request file stays one row.
pub(super) fn file_item(path: PathBuf, name: String) -> CollectionItem {
    let blocks = fs::read_to_string(&path)
        .ok()
        .and_then(|c| http_parser::parse_blocks(&c).ok())
        .unwrap_or_default();
    let children: Vec<CollectionItem> = if blocks.len() > 1 {
        blocks
            .iter()
            .enumerate()
            .map(|(i, block)| CollectionItem {
                name: block.label(),
                path: path.clone(),
                is_folder: false,
                children: Vec::new(),
                method: Some(request_badge(&block.request)),
                expanded: false,
                request_index: Some(i),
            })
            .collect()
    } else {
        Vec::new()
    };
    let method = blocks
        .first()
        .filter(|_| children.is_empty())
        .map(|b| request_badge(&b.request));
    CollectionItem {
        name,
        path,
        is_folder: false,
        children,
        method,
        expanded: false,
        request_index: None,
    }
}

impl ExplorerPanel {
    pub(super) fn scan_directory(&self, path: &PathBuf) -> Vec<CollectionItem> {
        debug!("Scanning: {}", path.display());
        let mut items = Vec::new();

        if let Ok(entries) = fs::read_dir(path) {
            let mut entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
            entries.sort_by(|a, b| {
                let a_is_dir = a.path().is_dir();
                let b_is_dir = b.path().is_dir();
                match (a_is_dir, b_is_dir) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.file_name().cmp(&b.file_name()),
                }
            });

            for entry in entries {
                let entry_path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();

                if name.starts_with('.') {
                    continue;
                }

                if entry_path.is_dir() {
                    let children = self.scan_directory(&entry_path);
                    if !children.is_empty() || self.folder_has_http_files(&entry_path) {
                        items.push(CollectionItem {
                            name,
                            path: entry_path,
                            is_folder: true,
                            children,
                            method: None,
                            expanded: false,
                            request_index: None,
                        });
                    }
                } else if entry_path.extension().is_some_and(|ext| ext == "http") {
                    items.push(file_item(entry_path, name));
                }
            }
        }

        items
    }

    fn folder_has_http_files(&self, path: &PathBuf) -> bool {
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.filter_map(|e| e.ok()) {
                let entry_path = entry.path();
                if entry_path.is_file() && entry_path.extension().is_some_and(|ext| ext == "http") {
                    return true;
                }
            }
        }
        false
    }
}
