//! Saving a request loaded from a multi request .http file.

use crate::panels::request::RequestPanel;
use crate::panels::response::ResponsePanel;
use crate::test_support::{TempDir, init_theme};
use gpui::{AppContext, Entity, TestAppContext, VisualTestContext};
use protide_core::execution::ws::TungsteniteExecutor;
use std::path::Path;

type TestPanel = RequestPanel<TungsteniteExecutor>;

const USERS: &str = "\
### List users
# @name list

GET {{base}}/users?limit=20

### Create user
# @name create-user

POST {{base}}/users
Content-Type: application/json

{\"name\": \"a\"}

# @set id = $.id

# @tests
expect(response.status).toBe(201);

### Login
# @name login

POST {{base}}/login
";

fn panel(cx: &mut TestAppContext) -> (Entity<TestPanel>, &mut VisualTestContext) {
    init_theme(cx);
    cx.add_window_view(|window, cx| {
        let response_panel = cx.new(|cx| ResponsePanel::new(window, cx));
        TestPanel::new(window, cx, response_panel)
    })
}

/// Load request `index` of `file` the way the explorer does, then render so
/// the queued editor contents land.
fn load(p: &Entity<TestPanel>, cx: &mut VisualTestContext, file: &Path, index: usize) {
    let content = std::fs::read_to_string(file).unwrap();
    let req = http_parser::parse(&content).unwrap().remove(index);
    p.update(cx, |p, cx| {
        p.load_from_parsed_request(&req, cx);
        p.set_loaded_file(file.to_path_buf(), index, req, content, cx);
    });
    cx.run_until_parked();
}

#[gpui::test]
async fn saving_the_middle_request_keeps_the_other_requests(cx: &mut TestAppContext) {
    let tmp = TempDir::new("save-multi");
    let file = tmp.write("users.http", USERS);
    let (p, cx) = panel(cx);
    load(&p, cx, &file, 1);

    p.update(cx, |p, cx| {
        p.url = "{{base}}/people".into();
        p.save_request(cx);
    });

    let saved = std::fs::read_to_string(&file).unwrap();
    let blocks = http_parser::parse_blocks(&saved).unwrap();
    assert_eq!(blocks.len(), 3, "no request may be dropped:\n{saved}");
    assert_eq!(blocks[0].request.url, "{{base}}/users?limit=20");
    assert_eq!(blocks[2].request.meta.name.as_deref(), Some("login"));
    let edited = &blocks[1];
    assert_eq!(edited.title.as_deref(), Some("Create user"));
    assert_eq!(edited.request.url, "{{base}}/people");
    assert_eq!(edited.request.meta.name.as_deref(), Some("create-user"));
    assert_eq!(edited.request.meta.variable_extractions[0].name, "id");
    assert!(edited.request.scripts.tests.is_some());
    assert_eq!(
        saved.replace("{{base}}/people", "{{base}}/users"),
        USERS,
        "only the edited URL may change"
    );
}

#[gpui::test]
async fn disabled_headers_survive_a_load_and_save(cx: &mut TestAppContext) {
    let tmp = TempDir::new("save-disabled");
    let text = "### One\n\nGET https://a.test/x\nAccept: */*\n# X-Debug: 1\nX-On: 2\n";
    let file = tmp.write("one.http", text);
    let (p, cx) = panel(cx);
    load(&p, cx, &file, 0);

    p.read_with(cx, |p, _| {
        let rows: Vec<_> = p
            .headers
            .iter()
            .filter(|h| !h.key.is_empty())
            .map(|h| (h.key.as_str(), h.enabled))
            .collect();
        assert_eq!(
            rows,
            vec![("Accept", true), ("X-Debug", false), ("X-On", true)]
        );
    });
    p.update(cx, |p, cx| p.save_request(cx));

    assert_eq!(std::fs::read_to_string(&file).unwrap(), text);
}

#[gpui::test]
async fn saving_an_unchanged_single_request_file_rewrites_nothing(cx: &mut TestAppContext) {
    let tmp = TempDir::new("save-single");
    // Has a body: a GET would currently inherit the default body (B5).
    let single = "### One\n# @name one\n\nPUT https://a.test/x\nAccept: */*\n\n{\"a\": 1}\n";
    let file = tmp.write("one.http", single);
    let (p, cx) = panel(cx);
    load(&p, cx, &file, 0);

    p.update(cx, |p, cx| p.save_request(cx));

    assert_eq!(std::fs::read_to_string(&file).unwrap(), single);
}

#[gpui::test]
async fn the_apps_own_save_is_not_an_external_change(cx: &mut TestAppContext) {
    let tmp = TempDir::new("save-watch");
    let file = tmp.write("users.http", USERS);
    let (p, cx) = panel(cx);
    load(&p, cx, &file, 2);

    p.update(cx, |p, cx| {
        p.url = "{{base}}/signin".into();
        p.save_request(cx);
    });
    let on_disk = std::fs::read_to_string(&file).unwrap();
    p.read_with(cx, |p, _| {
        assert!(!p.is_external_change(&file, Some(&on_disk)));
        assert!(p.is_external_change(&file, Some("GET https://other.test\n")));
        assert!(!p.is_external_change(&tmp.join("other.http"), Some("x")));
    });
}

#[gpui::test]
async fn an_edit_marks_the_request_dirty_until_it_is_saved(cx: &mut TestAppContext) {
    let tmp = TempDir::new("save-dirty");
    let file = tmp.write("users.http", USERS);
    let (p, cx) = panel(cx);
    load(&p, cx, &file, 1);
    p.read_with(cx, |p, _| assert_eq!(p.dirty_location(), None));

    p.update(cx, |p, cx| {
        p.url = "{{base}}/people".into();
        cx.notify();
    });
    cx.run_until_parked();
    p.read_with(cx, |p, _| {
        assert_eq!(p.dirty_location(), Some((file.clone(), 1)));
    });

    p.update(cx, |p, cx| assert!(p.save_request(cx)));
    cx.run_until_parked();
    p.read_with(cx, |p, _| assert_eq!(p.dirty_location(), None));
}
