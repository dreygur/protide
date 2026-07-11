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
            let line = req.line as u32;
            let range = Range {
                start: Position { line, character: 0 },
                end: Position { line, character: u32::MAX },
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
    let trimmed = line.trim_start();
    let old_name = parse_annotation_value(trimmed, "# @name")?;
    let indent = (line.len() - trimmed.len()) as u32;
    let name_start = indent + "# @name ".len() as u32;
    let name_end = name_start + old_name.len() as u32;
    Some(PrepareRenameResponse::RangeWithPlaceholder {
        range: Range {
            start: Position { line: pos.line, character: name_start },
            end: Position { line: pos.line, character: name_end },
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
                    start: Position { line: i as u32, character: 0 },
                    end: Position { line: i as u32, character: l.len() as u32 },
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
    Some(WorkspaceEdit { changes: Some(changes), ..Default::default() })
}

fn goto_named_request(content: &str, uri: &Url, name: &str) -> Option<GotoDefinitionResponse> {
    let requests = http_parser::parse(content).ok()?;
    let target = requests.iter().find(|r| r.meta.name.as_deref() == Some(name))?;
    let line = target.line as u32;
    Some(GotoDefinitionResponse::Scalar(Location {
        uri: uri.clone(),
        range: Range {
            start: Position { line, character: 0 },
            end: Position { line, character: u32::MAX },
        },
    }))
}

fn goto_variable(content: &str, uri: &Url, var_name: &str) -> Option<GotoDefinitionResponse> {
    let line_num = content.lines().enumerate().find_map(|(i, line)| {
        let rest = line.trim_start().strip_prefix("# @set")?;
        let rest = rest.trim_start();
        let name_end = rest.find(|c: char| c.is_whitespace() || c == '=').unwrap_or(rest.len());
        if &rest[..name_end] == var_name { Some(i as u32) } else { None }
    })?;
    Some(GotoDefinitionResponse::Scalar(Location {
        uri: uri.clone(),
        range: Range {
            start: Position { line: line_num, character: 0 },
            end: Position { line: line_num, character: u32::MAX },
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
