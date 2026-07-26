use crate::symbols::lsp_line;
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

            let line = lsp_line(req);
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
        let Ok(content) = std::fs::read_to_string(&file) else {
            continue;
        };
        let Ok(requests) = http_parser::parse(&content) else {
            continue;
        };
        let target = requests
            .iter()
            .find(|r| r.meta.name.as_deref() == Some(dep_name));
        if let Some(req) = target {
            let Ok(uri) = Url::from_file_path(&file) else {
                continue;
            };
            let line = lsp_line(req);
            return Some(GotoDefinitionResponse::Scalar(Location {
                uri,
                range: Range {
                    start: Position { line, character: 0 },
                    end: Position {
                        line,
                        character: u32::MAX,
                    },
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
                .is_some_and(|n| n.starts_with('.') || n == "target" || n == "node_modules");
            if !skip {
                collect_http_files(&path, out, depth - 1);
            }
        } else if path.extension().is_some_and(|e| e == "http") {
            out.push(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Unique scratch directory for one test - derived from pid + a monotonic
    /// timestamp so concurrent tests (and concurrent `cargo test` runs) never
    /// collide on a fixed path.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new() -> Self {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let dir =
                std::env::temp_dir().join(format!("protide-lsp-ws-{}-{nanos}", std::process::id()));
            std::fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }

        fn write(&self, rel: &str, content: &str) {
            let path = self.0.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, content).unwrap();
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn workspace_symbols_finds_named_requests_across_files() {
        let s = Scratch::new();
        s.write("a.http", "# @name Login\nPOST https://example.com/login\n");
        s.write(
            "nested/b.http",
            "# @name Profile\nGET https://example.com/me\n",
        );

        let mut names: Vec<String> = workspace_symbols(&s.0, "")
            .into_iter()
            .map(|s| s.name)
            .collect();
        names.sort();
        assert_eq!(names, vec!["Login", "Profile"]);
    }

    #[test]
    fn workspace_symbols_reports_the_zero_indexed_request_line() {
        let s = Scratch::new();
        s.write("a.http", "# @name Login\nPOST https://example.com/login\n");
        let syms = workspace_symbols(&s.0, "Login");
        assert_eq!(syms.len(), 1);
        assert_eq!(
            syms[0].location.range.start.line, 1,
            "`Request::line` is 1-indexed; LSP positions are 0-indexed",
        );
    }

    #[test]
    fn workspace_symbols_filters_case_insensitively_by_query() {
        let s = Scratch::new();
        s.write("a.http", "# @name Login\nPOST https://example.com/login\n");
        s.write("b.http", "# @name Profile\nGET https://example.com/me\n");

        let names: Vec<String> = workspace_symbols(&s.0, "prof")
            .into_iter()
            .map(|s| s.name)
            .collect();
        assert_eq!(names, vec!["Profile"]);
    }

    #[test]
    fn workspace_symbols_skips_non_http_files_hidden_dirs_and_unparseable_files() {
        let s = Scratch::new();
        s.write("a.http", "# @name Login\nPOST https://example.com/login\n");
        s.write("notes.txt", "# @name NotAHttpFile\nGET https://x.com\n");
        s.write(".hidden/c.http", "# @name Hidden\nGET https://x.com\n");
        s.write(
            "node_modules/d.http",
            "# @name Vendored\nGET https://x.com\n",
        );
        s.write("broken.http", "this is not a request\n");

        let names: Vec<String> = workspace_symbols(&s.0, "")
            .into_iter()
            .map(|s| s.name)
            .collect();
        assert_eq!(names, vec!["Login"]);
    }

    #[test]
    fn workspace_symbols_of_a_missing_directory_is_empty_not_a_panic() {
        let missing = std::env::temp_dir().join("protide-lsp-does-not-exist-xyz");
        assert!(workspace_symbols(&missing, "").is_empty());
    }

    #[test]
    fn workspace_goto_depends_locates_the_request_in_another_file() {
        let s = Scratch::new();
        s.write("a.http", "# @name Login\nPOST https://example.com/login\n");
        let Some(GotoDefinitionResponse::Scalar(loc)) = workspace_goto_depends(&s.0, "Login")
        else {
            panic!("expected to find Login");
        };
        assert!(loc.uri.path().ends_with("a.http"));
        assert_eq!(loc.range.start.line, 1);
    }

    #[test]
    fn workspace_goto_depends_returns_none_for_an_unknown_name() {
        let s = Scratch::new();
        s.write("a.http", "# @name Login\nPOST https://example.com/login\n");
        assert!(workspace_goto_depends(&s.0, "Nope").is_none());
    }
}
