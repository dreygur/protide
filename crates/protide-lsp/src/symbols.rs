use std::collections::HashMap;
use tower_lsp::lsp_types::*;

/// Convert a UTF-16 code-unit offset (as used by LSP's `Position.character`)
/// into a byte offset into `line`.
///
/// LSP positions count UTF-16 code units, not bytes and not chars. Using
/// `pos.character` directly as a byte index into a UTF-8 `&str` panics as
/// soon as a multi-byte character appears before the cursor; using it as a
/// char count silently misidentifies the position whenever an astral-plane
/// character (most emoji) appears before the cursor, since those encode as
/// two UTF-16 code units but one `char`. Walking `char_indices` and summing
/// each character's `len_utf16()` handles both cases correctly.
///
/// Returns `line.len()` (a valid byte offset, one past the last byte) if
/// `utf16_offset` is at or beyond the end of the line.
pub fn utf16_offset_to_byte_offset(line: &str, utf16_offset: usize) -> usize {
    let mut utf16_count = 0usize;
    for (byte_idx, c) in line.char_indices() {
        if utf16_count >= utf16_offset {
            return byte_idx;
        }
        utf16_count += c.len_utf16();
    }
    line.len()
}

/// 0-indexed LSP line for a parsed request.
///
/// `http_parser::Request::line` is the lexer's *1-indexed* source line (the
/// same convention `ParseError::line` uses, see `diagnostics.rs`). Handing it
/// to the client unconverted put every outline entry and every
/// go-to-definition jump one line below the request it names.
pub fn lsp_line(req: &http_parser::Request) -> u32 {
    req.line.saturating_sub(1) as u32
}

pub fn document_symbols(content: &str) -> Option<DocumentSymbolResponse> {
    let requests = http_parser::parse(content).ok()?;
    let symbols = requests
        .iter()
        .map(|req| {
            let name = req
                .meta
                .name
                .clone()
                .unwrap_or_else(|| format!("{} {}", req.method.as_str(), req.url));
            let line = lsp_line(req);
            let range = Range {
                start: Position { line, character: 0 },
                end: Position {
                    line,
                    character: u32::MAX,
                },
            };
            #[allow(deprecated)]
            DocumentSymbol {
                name,
                detail: Some(format!("{} {}", req.method.as_str(), req.url)),
                kind: SymbolKind::FUNCTION,
                range,
                selection_range: range,
                children: None,
                tags: None,
                deprecated: None,
            }
        })
        .collect();
    Some(DocumentSymbolResponse::Nested(symbols))
}

pub fn goto_definition_at(
    content: &str,
    uri: &Url,
    pos: Position,
) -> Option<GotoDefinitionResponse> {
    let line = content.lines().nth(pos.line as usize)?;
    let trimmed = line.trim_start();

    if let Some(dep_name) = parse_annotation_value(trimmed, "# @depends") {
        return goto_named_request(content, uri, dep_name);
    }

    if let Some(var_name) = var_at_cursor(line, pos.character as usize) {
        return goto_variable(content, uri, var_name);
    }

    None
}

pub fn prepare_rename_at(content: &str, pos: Position) -> Option<PrepareRenameResponse> {
    let line = content.lines().nth(pos.line as usize)?;
    let old_name = parse_annotation_value(line.trim_start(), "# @name")?;
    // `old_name` is a subslice of `line`, so the distance between the two
    // pointers *is* its start offset. Reconstructing it as
    // `indent + "# @name ".len()` assumed exactly one space after the
    // annotation, so `# @name  Greeting` (or a tab) handed the editor a range
    // shifted left of the actual name.
    let name_start = (old_name.as_ptr() as usize - line.as_ptr() as usize) as u32;
    let name_end = name_start + old_name.len() as u32;
    Some(PrepareRenameResponse::RangeWithPlaceholder {
        range: Range {
            start: Position {
                line: pos.line,
                character: name_start,
            },
            end: Position {
                line: pos.line,
                character: name_end,
            },
        },
        placeholder: old_name.to_string(),
    })
}

pub fn rename_symbol(
    content: &str,
    uri: &Url,
    pos: Position,
    new_name: &str,
) -> Option<WorkspaceEdit> {
    let line = content.lines().nth(pos.line as usize)?;
    let old_name = parse_annotation_value(line.trim_start(), "# @name")?;

    let mut edits = Vec::new();
    for (i, l) in content.lines().enumerate() {
        let t = l.trim_start();
        let is_name = parse_annotation_value(t, "# @name") == Some(old_name);
        let is_dep = parse_annotation_value(t, "# @depends") == Some(old_name);
        if is_name || is_dep {
            let prefix = if is_name { "# @name" } else { "# @depends" };
            let indent = l.len() - t.len();
            edits.push(TextEdit {
                range: Range {
                    start: Position {
                        line: i as u32,
                        character: 0,
                    },
                    end: Position {
                        line: i as u32,
                        character: l.len() as u32,
                    },
                },
                new_text: format!("{}{} {}", &l[..indent], prefix, new_name),
            });
        }
    }

    if edits.is_empty() {
        return None;
    }
    let mut changes = HashMap::new();
    changes.insert(uri.clone(), edits);
    Some(WorkspaceEdit {
        changes: Some(changes),
        ..Default::default()
    })
}

fn goto_named_request(content: &str, uri: &Url, name: &str) -> Option<GotoDefinitionResponse> {
    let requests = http_parser::parse(content).ok()?;
    let target = requests
        .iter()
        .find(|r| r.meta.name.as_deref() == Some(name))?;
    let line = lsp_line(target);
    Some(GotoDefinitionResponse::Scalar(Location {
        uri: uri.clone(),
        range: Range {
            start: Position { line, character: 0 },
            end: Position {
                line,
                character: u32::MAX,
            },
        },
    }))
}

fn goto_variable(content: &str, uri: &Url, var_name: &str) -> Option<GotoDefinitionResponse> {
    let line_num = content.lines().enumerate().find_map(|(i, line)| {
        let rest = line.trim_start().strip_prefix("# @set")?;
        let rest = rest.trim_start();
        let name_end = rest
            .find(|c: char| c.is_whitespace() || c == '=')
            .unwrap_or(rest.len());
        if &rest[..name_end] == var_name {
            Some(i as u32)
        } else {
            None
        }
    })?;
    Some(GotoDefinitionResponse::Scalar(Location {
        uri: uri.clone(),
        range: Range {
            start: Position {
                line: line_num,
                character: 0,
            },
            end: Position {
                line: line_num,
                character: u32::MAX,
            },
        },
    }))
}

/// `utf16_cursor` is a UTF-16 code-unit offset (e.g. LSP `Position.character`),
/// not a byte offset - it's converted internally so this never panics on
/// multi-byte characters preceding the cursor.
pub fn var_at_cursor(line: &str, utf16_cursor: usize) -> Option<&str> {
    let cursor = utf16_offset_to_byte_offset(line, utf16_cursor);
    let before = &line[..cursor];
    let open = before.rfind("{{")?;
    if before[open..].contains("}}") {
        return None;
    }
    let rest = &line[open + 2..];
    let close = rest.find("}}")?;
    let name = &rest[..close];
    if name.is_empty() { None } else { Some(name) }
}

pub fn parse_annotation_value<'a>(trimmed: &'a str, prefix: &str) -> Option<&'a str> {
    let rest = trimmed.strip_prefix(prefix)?;
    let value = rest.trim();
    if value.is_empty() { None } else { Some(value) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url() -> Url {
        Url::parse("file:///tmp/protide-test.http").unwrap()
    }

    fn pos(line: u32, character: u32) -> Position {
        Position { line, character }
    }

    // ── utf16_offset_to_byte_offset ──────────────────────────────────────────

    #[test]
    fn utf16_offset_is_identity_for_ascii() {
        assert_eq!(utf16_offset_to_byte_offset("GET /users", 0), 0);
        assert_eq!(utf16_offset_to_byte_offset("GET /users", 4), 4);
    }

    #[test]
    fn utf16_offset_accounts_for_two_byte_chars() {
        // 'é' is 2 UTF-8 bytes but a single UTF-16 code unit.
        assert_eq!(utf16_offset_to_byte_offset("hé!", 2), 3);
    }

    #[test]
    fn utf16_offset_accounts_for_astral_chars() {
        // '😀' is 4 UTF-8 bytes and *two* UTF-16 code units, so offset 2 is
        // the character right after it - a char-count would have said 1.
        assert_eq!(utf16_offset_to_byte_offset("😀ab", 2), 4);
        assert_eq!(utf16_offset_to_byte_offset("😀ab", 3), 5);
    }

    #[test]
    fn utf16_offset_past_end_clamps_to_line_length_without_panicking() {
        let line = "héllo";
        let byte = utf16_offset_to_byte_offset(line, 9_999);
        assert_eq!(byte, line.len());
        assert!(line.is_char_boundary(byte));
    }

    #[test]
    fn utf16_offset_always_lands_on_a_char_boundary() {
        let line = "GET https://ex.com/日本語/{{id}}";
        for i in 0..line.chars().count() * 2 + 5 {
            let byte = utf16_offset_to_byte_offset(line, i);
            assert!(
                line.is_char_boundary(byte),
                "offset {i} produced non-boundary byte index {byte}"
            );
        }
    }

    // ── var_at_cursor ────────────────────────────────────────────────────────

    #[test]
    fn var_at_cursor_finds_the_enclosing_variable() {
        assert_eq!(var_at_cursor("url {{token}} end", 8), Some("token"));
    }

    #[test]
    fn var_at_cursor_returns_none_once_the_braces_are_closed() {
        assert_eq!(var_at_cursor("url {{token}} end", 16), None);
    }

    #[test]
    fn var_at_cursor_returns_none_for_empty_braces() {
        assert_eq!(var_at_cursor("url {{}} end", 7), None);
    }

    #[test]
    fn var_at_cursor_handles_multibyte_prefix_without_panicking() {
        // Byte offsets and UTF-16 offsets diverge here; the cursor is 9 UTF-16
        // units in ("GET /日本語/{{" is 9 units up to just after "{{" + 2).
        let line = "GET /日本語/{{id}}";
        let units = line.chars().map(char::len_utf16).sum::<usize>();
        for i in 0..units + 3 {
            let _ = var_at_cursor(line, i);
        }
        // 12 UTF-16 units = "GET /日本語/{{i" -> cursor inside the variable.
        assert_eq!(var_at_cursor(line, 12), Some("id"));
    }

    // ── document_symbols ─────────────────────────────────────────────────────

    #[test]
    fn document_symbols_prefers_the_name_annotation() {
        let content = "# @name Greeting\nGET https://example.com/hi\n";
        let DocumentSymbolResponse::Nested(syms) = document_symbols(content).unwrap() else {
            panic!("expected nested symbols");
        };
        assert_eq!(syms.len(), 1);
        assert_eq!(syms[0].name, "Greeting");
        // 0-indexed: the request line is the second line of the document.
        assert_eq!(syms[0].range.start.line, 1);
        assert_eq!(
            syms[0].detail.as_deref(),
            Some("GET https://example.com/hi")
        );
    }

    #[test]
    fn document_symbols_falls_back_to_method_and_url_when_unnamed() {
        let content = "GET https://example.com/hi\n";
        let DocumentSymbolResponse::Nested(syms) = document_symbols(content).unwrap() else {
            panic!("expected nested symbols");
        };
        assert_eq!(syms[0].name, "GET https://example.com/hi");
        assert_eq!(syms[0].range.start.line, 0);
    }

    #[test]
    fn document_symbols_returns_none_for_unparseable_content() {
        assert!(document_symbols("this is not a request line\n").is_none());
    }

    #[test]
    fn document_symbols_of_empty_document_is_an_empty_list() {
        let DocumentSymbolResponse::Nested(syms) = document_symbols("").unwrap() else {
            panic!("expected nested symbols");
        };
        assert!(syms.is_empty());
    }

    // ── goto definition ──────────────────────────────────────────────────────

    #[test]
    fn goto_definition_on_depends_jumps_to_the_named_request() {
        let content = "\
# @name Login
POST https://example.com/login

###
# @depends Login
GET https://example.com/me
";
        let Some(GotoDefinitionResponse::Scalar(loc)) =
            goto_definition_at(content, &url(), pos(4, 4))
        else {
            panic!("expected a scalar location for the @depends target");
        };
        assert_eq!(loc.range.start.line, 1, "should point at the POST line");
    }

    #[test]
    fn goto_definition_on_a_variable_jumps_to_its_set_declaration() {
        let content = "\
# @name Login
# @set token = $.token
POST https://example.com/login

###
GET https://example.com/me?t={{token}}
";
        let Some(GotoDefinitionResponse::Scalar(loc)) =
            goto_definition_at(content, &url(), pos(5, 33))
        else {
            panic!("expected a scalar location for the variable");
        };
        assert_eq!(loc.range.start.line, 1);
    }

    #[test]
    fn goto_definition_past_end_of_document_is_none_not_a_panic() {
        assert!(goto_definition_at("GET https://x.com\n", &url(), pos(99, 99)).is_none());
    }

    #[test]
    fn goto_definition_on_an_unknown_dependency_is_none() {
        let content = "# @depends Nope\nGET https://example.com/me\n";
        assert!(goto_definition_at(content, &url(), pos(0, 12)).is_none());
    }

    // ── rename ───────────────────────────────────────────────────────────────

    #[test]
    fn prepare_rename_covers_exactly_the_name_token() {
        let content = "# @name Greeting\nGET https://example.com\n";
        let Some(PrepareRenameResponse::RangeWithPlaceholder { range, placeholder }) =
            prepare_rename_at(content, pos(0, 10))
        else {
            panic!("expected a rename range");
        };
        assert_eq!(placeholder, "Greeting");
        assert_eq!(range.start.character, 8);
        assert_eq!(range.end.character, 16);
        assert_eq!(
            &content.lines().next().unwrap()
                [range.start.character as usize..range.end.character as usize],
            "Greeting",
        );
    }

    #[test]
    fn prepare_rename_range_tracks_extra_whitespace_after_the_annotation() {
        // Regression: the range used to be hard-coded to `"# @name ".len()`,
        // so any extra space shifted it off the real name.
        let line = "# @name   Greeting";
        let Some(PrepareRenameResponse::RangeWithPlaceholder { range, placeholder }) =
            prepare_rename_at(&format!("{line}\nGET https://example.com\n"), pos(0, 12))
        else {
            panic!("expected a rename range");
        };
        assert_eq!(placeholder, "Greeting");
        assert_eq!(
            &line[range.start.character as usize..range.end.character as usize],
            "Greeting",
            "the rename range must cover the name itself, not the whitespace before it",
        );
    }

    #[test]
    fn prepare_rename_range_accounts_for_indentation() {
        let line = "  # @name Greeting";
        let Some(PrepareRenameResponse::RangeWithPlaceholder { range, .. }) =
            prepare_rename_at(&format!("{line}\nGET https://example.com\n"), pos(0, 12))
        else {
            panic!("expected a rename range");
        };
        assert_eq!(
            &line[range.start.character as usize..range.end.character as usize],
            "Greeting",
        );
    }

    #[test]
    fn prepare_rename_on_a_non_name_line_is_none() {
        let content = "GET https://example.com\n";
        assert!(prepare_rename_at(content, pos(0, 2)).is_none());
        assert!(prepare_rename_at(content, pos(50, 2)).is_none());
    }

    #[test]
    fn prepare_rename_on_an_empty_name_is_none() {
        assert!(prepare_rename_at("# @name\nGET https://x.com\n", pos(0, 7)).is_none());
    }

    #[test]
    fn rename_rewrites_both_the_declaration_and_every_dependent() {
        let content = "\
# @name Login
POST https://example.com/login

###
# @depends Login
GET https://example.com/me
";
        let edit = rename_symbol(content, &url(), pos(0, 10), "SignIn").unwrap();
        let edits = &edit.changes.unwrap()[&url()];
        assert_eq!(edits.len(), 2, "declaration + one @depends reference");
        assert_eq!(edits[0].new_text, "# @name SignIn");
        assert_eq!(edits[1].new_text, "# @depends SignIn");
        assert_eq!(edits[1].range.start.line, 4);
    }

    #[test]
    fn rename_preserves_indentation() {
        let content = "  # @name Login\n  POST https://example.com/login\n";
        let edit = rename_symbol(content, &url(), pos(0, 12), "SignIn").unwrap();
        let edits = &edit.changes.unwrap()[&url()];
        assert_eq!(edits[0].new_text, "  # @name SignIn");
    }

    #[test]
    fn rename_from_a_non_name_line_is_none() {
        let content = "# @name Login\nPOST https://example.com/login\n";
        assert!(rename_symbol(content, &url(), pos(1, 2), "SignIn").is_none());
        assert!(rename_symbol(content, &url(), pos(99, 2), "SignIn").is_none());
    }

    // ── parse_annotation_value ───────────────────────────────────────────────

    #[test]
    fn parse_annotation_value_trims_and_rejects_empties() {
        assert_eq!(
            parse_annotation_value("# @name  Foo ", "# @name"),
            Some("Foo")
        );
        assert_eq!(parse_annotation_value("# @name   ", "# @name"), None);
        assert_eq!(parse_annotation_value("# @other Foo", "# @name"), None);
    }
}
