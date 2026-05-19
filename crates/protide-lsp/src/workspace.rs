use std::path::Path;
use tower_lsp::lsp_types::*;

pub fn workspace_symbols(root: &Path, query: &str) -> Vec<SymbolInformation> {
    let mut files = Vec::new();
    collect_http_files(root, &mut files, 8);

    let query_lower = query.to_lowercase();
    let mut symbols = Vec::new();

    for file in files {
        let Ok(content) = std::fs::read_to_string(&file) else {
            continue;
        };
        let Ok(requests) = http_parser::parse(&content) else {
            continue;
        };
        let Ok(uri) = Url::from_file_path(&file) else {
            continue;
        };

        for req in &requests {
            let name = req
                .meta
                .name
                .clone()
                .unwrap_or_else(|| format!("{} {}", req.method.as_str(), req.url));

            if !query_lower.is_empty() && !name.to_lowercase().contains(&query_lower) {
                continue;
            }

            let line = req.line as u32;
            #[allow(deprecated)]
            symbols.push(SymbolInformation {
                name,
                kind: SymbolKind::FUNCTION,
                tags: None,
                deprecated: None,
                location: Location {
                    uri: uri.clone(),
                    range: Range {
                        start: Position { line, character: 0 },
                        end: Position { line, character: 0 },
                    },
                },
                container_name: None,
            });
        }
    }
    symbols
}

/// Search all .http files in the workspace for a request named `dep_name`.
pub fn workspace_goto_depends(root: &Path, dep_name: &str) -> Option<GotoDefinitionResponse> {
    let mut files = Vec::new();
    collect_http_files(root, &mut files, 8);

    for file in files {
        let Ok(content) = std::fs::read_to_string(&file) else { continue };
        let Ok(requests) = http_parser::parse(&content) else { continue };
        let target = requests.iter().find(|r| r.meta.name.as_deref() == Some(dep_name));
        if let Some(req) = target {
            let Ok(uri) = Url::from_file_path(&file) else { continue };
            let line = req.line as u32;
            return Some(GotoDefinitionResponse::Scalar(Location {
                uri,
                range: Range {
                    start: Position { line, character: 0 },
                    end: Position { line, character: u32::MAX },
                },
            }));
        }
    }
    None
}

fn collect_http_files(dir: &Path, out: &mut Vec<std::path::PathBuf>, depth: usize) {
    if depth == 0 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let skip = path
                .file_name()
                .and_then(|n| n.to_str())
                .map_or(false, |n| n.starts_with('.') || n == "target" || n == "node_modules");
            if !skip {
                collect_http_files(&path, out, depth - 1);
            }
        } else if path.extension().map_or(false, |e| e == "http") {
            out.push(path);
        }
    }
}
