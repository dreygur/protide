use log::debug;
use super::*;

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
                        });
                    }
                } else if entry_path.extension().is_some_and(|ext| ext == "http") {
                    let method = self.parse_method_from_file(&entry_path);
                    items.push(CollectionItem {
                        name,
                        path: entry_path,
                        is_folder: false,
                        children: Vec::new(),
                        method,
                        expanded: false,
                    });
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

    pub(super) fn parse_method_from_file(&self, path: &PathBuf) -> Option<String> {
        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(requests) = http_parser::parse(&content) {
                if let Some(req) = requests.first() {
                    let label = match req.protocol() {
                        Protocol::GraphQL => "GQL",
                        Protocol::WebSocket => "WS",
                        Protocol::Grpc => "GRPC",
                        Protocol::SocketIO => "SIO",
                        Protocol::Trpc => "TRPC",
                        Protocol::Http => req.method.as_str(),
                    };
                    return Some(label.to_string());
                }
            }
        }
        None
    }
}
