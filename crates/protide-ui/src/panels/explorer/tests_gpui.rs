//! Tree scan / navigation tests for [`ExplorerPanel`].

use super::{CollectionItem, ExplorerPanel};
use crate::test_support::TempDir;
use gpui::{AppContext as _, Entity, TestAppContext, WeakEntity};
use std::path::{Path, PathBuf};

/// An `ExplorerPanel` with no `MainWindow` behind it - none of the tree code
/// under test reaches back through that handle.
fn panel(cx: &mut TestAppContext) -> Entity<ExplorerPanel> {
    cx.new(|cx| ExplorerPanel::new(cx, WeakEntity::new_invalid()))
}

fn write(path: &Path, contents: &str) {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).unwrap();
    }
    std::fs::write(path, contents).unwrap();
}

fn names(items: &[CollectionItem]) -> Vec<&str> {
    items.iter().map(|i| i.name.as_str()).collect()
}

fn find<'a>(items: &'a [CollectionItem], name: &str) -> &'a CollectionItem {
    items
        .iter()
        .find(|i| i.name == name)
        .unwrap_or_else(|| panic!("no item named {name:?} in {:?}", names(items)))
}

// ── scan_directory ──────────────────────────────────────────────────────────

#[gpui::test]
fn only_http_files_and_folders_that_contain_them_appear_in_the_tree(cx: &mut TestAppContext) {
    let tmp = TempDir::new("scan");
    let root = tmp.path();
    write(&root.join("a.http"), "GET https://x.test\n");
    write(&root.join("notes.md"), "# not a request");
    write(&root.join("no-ext"), "");
    std::fs::create_dir_all(root.join("empty-folder")).unwrap();
    write(&root.join("nested/b.http"), "POST https://x.test\n");
    write(&root.join("docs-only/readme.md"), "hi");

    let p = panel(cx);
    let items = p.read_with(cx, |p, _| p.scan_directory(&root.to_path_buf()));

    assert_eq!(names(&items), vec!["nested", "a.http"]);
    assert!(find(&items, "nested").is_folder);
    assert!(!find(&items, "a.http").is_folder);
}

#[gpui::test]
fn folders_are_listed_before_files(cx: &mut TestAppContext) {
    let tmp = TempDir::new("scan-order");
    let root = tmp.path();
    // Named so that plain alphabetical order would interleave them.
    write(&root.join("aaa.http"), "GET https://x.test\n");
    write(&root.join("zzz.http"), "GET https://x.test\n");
    write(&root.join("mmm/x.http"), "GET https://x.test\n");

    let p = panel(cx);
    let items = p.read_with(cx, |p, _| p.scan_directory(&root.to_path_buf()));
    assert_eq!(names(&items), vec!["mmm", "aaa.http", "zzz.http"]);
}

#[gpui::test]
fn hidden_entries_are_skipped(cx: &mut TestAppContext) {
    let tmp = TempDir::new("scan-hidden");
    let root = tmp.path();
    write(&root.join(".hidden.http"), "GET https://x.test\n");
    write(&root.join(".git/config.http"), "GET https://x.test\n");
    write(&root.join("visible.http"), "GET https://x.test\n");

    let p = panel(cx);
    let items = p.read_with(cx, |p, _| p.scan_directory(&root.to_path_buf()));
    assert_eq!(names(&items), vec!["visible.http"]);
}

#[gpui::test]
fn nested_requests_are_scanned_recursively(cx: &mut TestAppContext) {
    let tmp = TempDir::new("scan-deep");
    let root = tmp.path();
    write(&root.join("a/b/c/deep.http"), "GET https://x.test\n");

    let p = panel(cx);
    let items = p.read_with(cx, |p, _| p.scan_directory(&root.to_path_buf()));
    let a = find(&items, "a");
    let b = find(&a.children, "b");
    let c = find(&b.children, "c");
    assert_eq!(names(&c.children), vec!["deep.http"]);
}

#[gpui::test]
fn scanning_a_missing_or_unreadable_directory_yields_nothing(cx: &mut TestAppContext) {
    let tmp = TempDir::new("scan-missing");
    let p = panel(cx);
    p.read_with(cx, |p, _| {
        assert!(p.scan_directory(&tmp.join("does-not-exist")).is_empty());
        // A *file* where a directory was expected must not panic either.
        let file = tmp.write("a.http", "GET https://x.test\n");
        assert!(p.scan_directory(&file).is_empty());
    });
}

#[gpui::test]
fn unicode_file_and_folder_names_survive_the_scan(cx: &mut TestAppContext) {
    let tmp = TempDir::new("scan-unicode");
    let root = tmp.path();
    write(&root.join("日本語/リクエスト.http"), "GET https://x.test\n");

    let p = panel(cx);
    let items = p.read_with(cx, |p, _| p.scan_directory(&root.to_path_buf()));
    assert_eq!(names(&items), vec!["日本語"]);
    assert_eq!(
        names(&find(&items, "日本語").children),
        vec!["リクエスト.http"]
    );
}

// ── file_item ───────────────────────────────────────────────────────────────

fn badge(path: &Path) -> Option<String> {
    super::tree_scan::file_item(path.to_path_buf(), "r.http".into()).method
}

#[gpui::test]
fn the_tree_badge_shows_the_method_or_protocol_of_a_single_request(_cx: &mut TestAppContext) {
    let tmp = TempDir::new("badge");
    for (body, want) in [
        ("GET https://x.test\n", Some("GET")),
        ("DELETE https://x.test\n", Some("DELETE")),
        ("# @protocol graphql\nPOST https://x.test\n", Some("GQL")),
        ("# @protocol websocket\nGET wss://x.test\n", Some("WS")),
        ("# @protocol grpc\nPOST https://x.test\n", Some("GRPC")),
    ] {
        let file = tmp.join("r.http");
        write(&file, body);
        assert_eq!(badge(&file).as_deref(), want, "for {body:?}");
    }
}

#[gpui::test]
fn an_unparseable_or_missing_file_has_no_method_badge(_cx: &mut TestAppContext) {
    let tmp = TempDir::new("badge-bad");
    assert_eq!(badge(&tmp.join("nope.http")), None);
    assert_eq!(badge(&tmp.write("empty.http", "")), None);
    assert_eq!(
        badge(&tmp.write("comment.http", "# just a comment\n")),
        None
    );
}

#[gpui::test]
fn a_multi_request_file_lists_every_request_as_a_child(_cx: &mut TestAppContext) {
    let tmp = TempDir::new("multi");
    let file = tmp.write(
        "users.http",
        "### List\nGET https://x.test\n\n###\n# @name drop\nDELETE https://x.test/1\n",
    );
    let item = super::tree_scan::file_item(file.clone(), "users.http".into());
    assert!(item.is_expandable());
    assert_eq!(item.method, None);
    let rows: Vec<_> = item
        .children
        .iter()
        .map(|c| {
            (
                c.name.as_str(),
                c.method.as_deref(),
                c.request_index,
                &c.path,
            )
        })
        .collect();
    assert_eq!(
        rows,
        vec![
            ("List", Some("GET"), Some(0), &file),
            ("drop", Some("DELETE"), Some(1), &file),
        ]
    );
}

#[gpui::test]
fn a_single_request_file_stays_one_row(_cx: &mut TestAppContext) {
    let tmp = TempDir::new("single");
    let file = tmp.write("one.http", "GET https://x.test\n");
    let item = super::tree_scan::file_item(file, "one.http".into());
    assert!(!item.is_expandable());
    assert!(item.children.is_empty());
}

// ── flatten / expand / collapse ─────────────────────────────────────────────

fn item(name: &str, is_folder: bool, children: Vec<CollectionItem>) -> CollectionItem {
    CollectionItem {
        name: name.to_string(),
        path: PathBuf::from(format!("/ws/{name}")),
        is_folder,
        children,
        method: None,
        expanded: false,
        request_index: None,
    }
}

/// `/ws/outer` -> `/ws/inner` -> `/ws/leaf.http`, plus a sibling `/ws/top.http`.
fn nested_tree() -> Vec<CollectionItem> {
    vec![
        item(
            "outer",
            true,
            vec![item("inner", true, vec![item("leaf.http", false, vec![])])],
        ),
        item("top.http", false, vec![]),
    ]
}

#[gpui::test]
fn a_collapsed_tree_flattens_to_its_top_level_only(cx: &mut TestAppContext) {
    let p = panel(cx);
    p.update(cx, |p, _| p.collection_items = nested_tree());
    let flat = p.read_with(cx, |p, _| {
        p.flatten_collection_items(&p.collection_items, 0)
    });
    assert_eq!(
        flat.iter()
            .map(|(i, d)| (i.name.as_str(), *d))
            .collect::<Vec<_>>(),
        vec![("outer", 0), ("top.http", 0)]
    );
}

#[gpui::test]
fn expanding_a_folder_reveals_its_children_at_the_next_depth(cx: &mut TestAppContext) {
    let p = panel(cx);
    p.update(cx, |p, cx| {
        p.collection_items = nested_tree();
        p.toggle_collection_folder(PathBuf::from("/ws/outer"), cx);
        p.toggle_collection_folder(PathBuf::from("/ws/inner"), cx);
    });
    let flat = p.read_with(cx, |p, _| {
        p.flatten_collection_items(&p.collection_items, 0)
    });
    assert_eq!(
        flat.iter()
            .map(|(i, d)| (i.name.as_str(), *d))
            .collect::<Vec<_>>(),
        vec![
            ("outer", 0),
            ("inner", 1),
            ("leaf.http", 2),
            ("top.http", 0),
        ]
    );
}

#[gpui::test]
fn toggling_a_folder_twice_returns_it_to_collapsed(cx: &mut TestAppContext) {
    let p = panel(cx);
    p.update(cx, |p, cx| {
        p.collection_items = nested_tree();
        p.toggle_collection_folder(PathBuf::from("/ws/outer"), cx);
        assert!(p.collection_items[0].expanded);
        p.toggle_collection_folder(PathBuf::from("/ws/outer"), cx);
        assert!(!p.collection_items[0].expanded);
    });
}

#[gpui::test]
fn toggling_a_file_or_an_unknown_path_changes_nothing(cx: &mut TestAppContext) {
    let p = panel(cx);
    p.update(cx, |p, cx| {
        p.collection_items = nested_tree();
        p.toggle_collection_folder(PathBuf::from("/ws/top.http"), cx);
        p.toggle_collection_folder(PathBuf::from("/ws/gone"), cx);
        p.toggle_collection_folder(PathBuf::new(), cx);
        let flat = p.flatten_collection_items(&p.collection_items, 0);
        assert_eq!(flat.len(), 2, "no folder should have opened");
    });
}

#[gpui::test]
fn collapse_all_closes_nested_folders_too(cx: &mut TestAppContext) {
    let p = panel(cx);
    p.update(cx, |p, cx| {
        p.collection_items = nested_tree();
        p.toggle_collection_folder(PathBuf::from("/ws/outer"), cx);
        p.toggle_collection_folder(PathBuf::from("/ws/inner"), cx);
        assert_eq!(p.flatten_collection_items(&p.collection_items, 0).len(), 4);

        p.collapse_all_folders(cx);
        assert_eq!(
            p.flatten_collection_items(&p.collection_items, 0).len(),
            2,
            "collapse-all must reach folders nested inside other folders"
        );
        assert!(!p.collection_items[0].children[0].expanded);
    });
}

#[gpui::test]
fn collapsing_the_collections_section_is_a_toggle(cx: &mut TestAppContext) {
    let p = panel(cx);
    p.update(cx, |p, cx| {
        let start = p.collections_expanded;
        p.toggle_collections(cx);
        assert_eq!(p.collections_expanded, !start);
        p.toggle_collections(cx);
        assert_eq!(p.collections_expanded, start);
    });
}

fn key(keys: &str) -> gpui::KeyDownEvent {
    gpui::KeyDownEvent {
        keystroke: gpui::Keystroke::parse(keys).unwrap(),
        is_held: false,
        prefer_character_input: false,
    }
}

/// REGRESSION: env inputs only honoured ctrl, so cmd-a and cmd-v did
/// nothing on macOS.
#[gpui::test]
fn env_inputs_select_all_and_paste_with_the_platform_modifier(cx: &mut TestAppContext) {
    let p = panel(cx);
    cx.write_to_clipboard(gpui::ClipboardItem::new_string("staging".into()));
    p.update(cx, |p, cx| {
        p.new_env_name = "dev".into();
        p.active_edit = Some(super::EnvEditTarget::NewEnvName);
        p.edit_selection = 3..3;
        for keys in ["secondary-a", "secondary-v"] {
            p.handle_edit_key(&key(keys), cx);
        }
        assert_eq!(p.new_env_name, "staging");
    });
}

#[gpui::test]
fn a_shortcut_letter_is_never_typed_into_an_env_input(cx: &mut TestAppContext) {
    let p = panel(cx);
    p.update(cx, |p, cx| {
        p.active_edit = Some(super::EnvEditTarget::NewEnvName);
        p.handle_edit_key(&key("secondary-q->q"), cx);
        assert_eq!(p.new_env_name, "");
    });
}

#[gpui::test]
async fn opening_another_request_never_drops_unsaved_edits(cx: &mut TestAppContext) {
    use crate::panels::request::RequestPanel;
    use crate::panels::response::ResponsePanel;

    crate::test_support::init_theme(cx);
    let tmp = TempDir::new("guard");
    let a = tmp.write("a.http", "GET https://a.test\n");
    let b = tmp.write("b.http", "GET https://b.test\n");
    let (rp, cx) = cx.add_window_view(|window, cx| {
        let response = cx.new(|cx| ResponsePanel::new(window, cx));
        RequestPanel::new(window, cx, response)
    });
    let ex = cx.new(|cx| ExplorerPanel::new(cx, WeakEntity::new_invalid()));
    ex.update(cx, |e, cx| e.set_request_panel(rp.clone(), cx));
    ex.update(cx, |e, cx| e.open_request(a.clone(), None, cx));
    cx.run_until_parked();
    rp.update(cx, |p, cx| {
        p.url = "https://edited.test".into();
        cx.notify();
    });
    cx.run_until_parked();

    // With no window to ask in, nothing may load over the edits.
    ex.update(cx, |e, cx| e.open_request(b.clone(), None, cx));
    ex.update(cx, |e, cx| e.open_request(a.clone(), None, cx));
    rp.read_with(cx, |p, _| {
        assert_eq!(p.url, "https://edited.test");
        assert_eq!(p.loaded_location(), Some((a.clone(), 0)));
    });
}
