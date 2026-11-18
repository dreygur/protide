/// Fallback used when a name sanitizes down to something unusable.
const FALLBACK_NAME: &str = "untitled";

/// Sanitize a string to be used as a single filename component.
///
/// The result is always a plain name that stays inside the directory it is
/// joined onto: separators become `-`, and the pure-navigation components
/// `.`/`..` (and the empty string) are replaced outright.
pub fn sanitize_filename(name: &str) -> String {
    let cleaned = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            _ => c,
        })
        .collect::<String>()
        .trim()
        .to_string();
    // `""`, `.` and `..` are not names - joining them onto a directory either
    // fails or escapes it, letting an imported collection write outside the
    // workspace.
    if matches!(cleaned.as_str(), "" | "." | "..") {
        return FALLBACK_NAME.to_string();
    }
    cleaned
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Component, Path, PathBuf};

    // ── sanitize_filename ────────────────────────────────────────────────────

    #[test]
    fn ordinary_names_are_left_alone() {
        for name in [
            "users",
            "Get User",
            "user-42",
            "日本語のリクエスト",
            "a.b.c",
        ] {
            assert_eq!(sanitize_filename(name), name);
        }
    }

    #[test]
    fn path_separators_and_reserved_characters_become_dashes() {
        assert_eq!(sanitize_filename("a/b"), "a-b");
        assert_eq!(sanitize_filename("a\\b"), "a-b");
        assert_eq!(sanitize_filename("C:\\Users"), "C--Users");
        assert_eq!(sanitize_filename("who?"), "who-");
        assert_eq!(sanitize_filename("a*b\"c<d>e|f"), "a-b-c-d-e-f");
    }

    #[test]
    fn surrounding_whitespace_is_trimmed() {
        assert_eq!(sanitize_filename("  users  "), "users");
        assert_eq!(sanitize_filename("\t\nusers\n"), "users");
    }

    // FIXED: `sanitize_filename` replaced separators but let `.` and `..`
    // through untouched, and could return an empty string. Import writes
    // `output_dir.join(sanitize_filename(collection_name))` and
    // `collection_dir.join(sanitize_filename(folder_name))`
    // (explorer/workspace_io.rs:37,53), so an imported Postman/OpenAPI
    // collection containing a folder literally named ".." wrote its .http files
    // one directory *above* the workspace - a directory-traversal write driven
    // by untrusted file content. An empty name was almost as bad: it produced a
    // dotfile (".http") that `scan_directory` then hides, so the import looked
    // like it had silently done nothing.
    #[test]
    fn navigation_components_never_escape_the_target_directory() {
        for name in ["..", ".", "", "   ", "../", "..\\", "/..", "  ..  "] {
            let sanitized = sanitize_filename(name);
            assert!(!sanitized.is_empty(), "{name:?} sanitized to an empty name");
            let joined = Path::new("/workspace/collection").join(&sanitized);
            assert!(
                joined.starts_with("/workspace/collection"),
                "{name:?} -> {sanitized:?} escaped to {}",
                joined.display()
            );
            assert!(
                !joined
                    .components()
                    .any(|c| matches!(c, Component::ParentDir | Component::CurDir)),
                "{name:?} -> {sanitized:?} kept a navigation component"
            );
        }
    }

    #[test]
    fn a_sanitized_name_is_always_exactly_one_path_component() {
        for name in [
            "a/b/c",
            "../../etc/passwd",
            "/absolute/path",
            "C:\\Windows\\System32",
            "..",
            "",
            "n\u{00e9}sted/na\u{0301}me",
        ] {
            let sanitized = sanitize_filename(name);
            let path = PathBuf::from(&sanitized);
            assert_eq!(
                path.components().count(),
                1,
                "{name:?} -> {sanitized:?} is not a single component"
            );
        }
    }
}
