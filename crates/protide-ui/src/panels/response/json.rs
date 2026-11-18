use super::*;

// ─── JSON tree types ────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub enum PrimVal {
    Null,
    Bool(bool),
    /// Pre-formatted display string, e.g. "42.5". Raw value == display for numbers.
    Num(SharedString),
    /// `raw` is the original JSON string value (used for clipboard copy).
    /// `display` is pre-sanitized and quoted: `"\"hello world\""`.
    Str {
        raw: String,
        display: SharedString,
    },
    EmptyArr,
    EmptyObj,
}

#[derive(Clone, Debug)]
pub enum RowKind {
    Leaf {
        key: Option<String>,
        val: PrimVal,
        path: String,
    },
    Open {
        key: Option<String>,
        arr: bool,
        path: String,
    },
    Close {
        arr: bool,
    },
    Folded {
        key: Option<String>,
        arr: bool,
        count: usize,
        path: String,
    },
}

/// Items shown in the JSON right-click context menu
#[derive(Clone, Debug)]
pub struct JsonCtxMenu {
    pub x: f32,
    pub y: f32,
    pub items: Vec<(String, String)>, // (label, clipboard_value)
}

#[derive(Clone, Debug)]
pub struct JsonRow {
    pub depth: usize,
    pub kind: RowKind,
}

pub fn flatten_json(
    value: &serde_json::Value,
    depth: usize,
    key: Option<String>,
    path: &str,
    collapsed: &std::collections::HashSet<String>,
    rows: &mut Vec<JsonRow>,
) {
    use serde_json::Value;
    let leaf_path = path.to_string();
    match value {
        Value::Null => rows.push(JsonRow {
            depth,
            kind: RowKind::Leaf {
                key,
                val: PrimVal::Null,
                path: leaf_path,
            },
        }),
        Value::Bool(b) => rows.push(JsonRow {
            depth,
            kind: RowKind::Leaf {
                key,
                val: PrimVal::Bool(*b),
                path: leaf_path,
            },
        }),
        Value::Number(n) => rows.push(JsonRow {
            depth,
            kind: RowKind::Leaf {
                key,
                val: PrimVal::Num(SharedString::new(n.to_string())),
                path: leaf_path,
            },
        }),
        Value::String(s) => {
            let san: String = s
                .chars()
                .map(|c| if c.is_control() { ' ' } else { c })
                .collect();
            let display = SharedString::new(format!("\"{}\"", san));
            rows.push(JsonRow {
                depth,
                kind: RowKind::Leaf {
                    key,
                    val: PrimVal::Str {
                        raw: s.clone(),
                        display,
                    },
                    path: leaf_path,
                },
            });
        }
        Value::Array(arr) if arr.is_empty() => {
            rows.push(JsonRow {
                depth,
                kind: RowKind::Leaf {
                    key,
                    val: PrimVal::EmptyArr,
                    path: leaf_path,
                },
            });
        }
        Value::Array(arr) => {
            let count = arr.len();
            let p = path.to_string();
            if collapsed.contains(path) {
                rows.push(JsonRow {
                    depth,
                    kind: RowKind::Folded {
                        key,
                        arr: true,
                        count,
                        path: p,
                    },
                });
                return;
            }
            rows.push(JsonRow {
                depth,
                kind: RowKind::Open {
                    key,
                    arr: true,
                    path: p,
                },
            });
            for (i, item) in arr.iter().enumerate() {
                flatten_json(
                    item,
                    depth + 1,
                    None,
                    &format!("{}/{}", path, i),
                    collapsed,
                    rows,
                );
            }
            rows.push(JsonRow {
                depth,
                kind: RowKind::Close { arr: true },
            });
        }
        Value::Object(obj) if obj.is_empty() => {
            rows.push(JsonRow {
                depth,
                kind: RowKind::Leaf {
                    key,
                    val: PrimVal::EmptyObj,
                    path: leaf_path,
                },
            });
        }
        Value::Object(obj) => {
            let count = obj.len();
            let p = path.to_string();
            if collapsed.contains(path) {
                rows.push(JsonRow {
                    depth,
                    kind: RowKind::Folded {
                        key,
                        arr: false,
                        count,
                        path: p,
                    },
                });
                return;
            }
            rows.push(JsonRow {
                depth,
                kind: RowKind::Open {
                    key,
                    arr: false,
                    path: p,
                },
            });
            for (k, v) in obj {
                flatten_json(
                    v,
                    depth + 1,
                    Some(k.clone()),
                    &format!("{}/{}", path, k),
                    collapsed,
                    rows,
                );
            }
            rows.push(JsonRow {
                depth,
                kind: RowKind::Close { arr: false },
            });
        }
    }
}

impl ResponsePanel {
    pub(super) fn toggle_json_collapse(&mut self, path: String, cx: &mut Context<Self>) {
        if self.json_tree_collapsed.contains(&path) {
            self.json_tree_collapsed.remove(&path);
        } else {
            self.json_tree_collapsed.insert(path);
        }
        self.json_sel = None;
        self.rebuild_json_rows();
        cx.notify();
    }

    pub(super) fn rebuild_json_rows(&mut self) {
        self.json_rows.clear();
        if let Some(json) = &self.json_value {
            flatten_json(
                json,
                0,
                None,
                "",
                &self.json_tree_collapsed,
                &mut self.json_rows,
            );
        }
    }
}
