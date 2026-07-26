use std::collections::HashMap;
use tower_lsp::lsp_types::*;

pub fn inlay_hints(content: &str, range: Range) -> Vec<InlayHint> {
    let set_vars = collect_set_vars(content);
    if set_vars.is_empty() {
        return vec![];
    }

    let start_line = range.start.line as usize;
    let end_line = range.end.line as usize;
    let mut hints = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        if line_num < start_line || line_num > end_line {
            continue;
        }
        scan_line_vars(line, line_num as u32, &set_vars, &mut hints);
    }

    hints
}

fn collect_set_vars(content: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in content.lines() {
        let Some(rest) = line.trim_start().strip_prefix("# @set") else {
            continue;
        };
        let rest = rest.trim_start();
        let Some(eq) = rest.find('=') else { continue };
        let name = rest[..eq].trim().to_string();
        let expr = rest[eq + 1..].trim().to_string();
        if !name.is_empty() {
            map.insert(name, expr);
        }
    }
    map
}

fn scan_line_vars(
    line: &str,
    line_num: u32,
    set_vars: &HashMap<String, String>,
    hints: &mut Vec<InlayHint>,
) {
    let mut search = line;
    let mut offset = 0usize;

    while let Some(open) = search.find("{{") {
        search = &search[open + 2..];
        offset += open + 2;

        let Some(close) = search.find("}}") else {
            break;
        };
        let var_name = &search[..close];

        if let Some(expr) = set_vars.get(var_name) {
            // LSP positions count UTF-16 code units, not bytes: emitting the
            // raw byte offset drifted the hint right by one column per extra
            // UTF-8 byte earlier in the line.
            let byte_pos = offset + close + 2;
            let char_pos = line[..byte_pos].chars().map(char::len_utf16).sum::<usize>() as u32;
            hints.push(InlayHint {
                position: Position {
                    line: line_num,
                    character: char_pos,
                },
                label: InlayHintLabel::String(format!("= {expr}")),
                kind: Some(InlayHintKind::PARAMETER),
                text_edits: None,
                tooltip: None,
                padding_left: Some(true),
                padding_right: None,
                data: None,
            });
        }

        search = &search[close + 2..];
        offset += close + 2;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn whole_doc() -> Range {
        Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: u32::MAX,
                character: 0,
            },
        }
    }

    fn label(h: &InlayHint) -> &str {
        match &h.label {
            InlayHintLabel::String(s) => s,
            _ => panic!("expected a string label"),
        }
    }

    #[test]
    fn a_variable_with_a_set_declaration_gets_its_expression_as_a_hint() {
        let content = "# @set token = $.data.token\nGET https://x.com?t={{token}}\n";
        let hints = inlay_hints(content, whole_doc());
        assert_eq!(hints.len(), 1);
        assert_eq!(label(&hints[0]), "= $.data.token");
        assert_eq!(hints[0].position.line, 1);
        // Hint sits immediately after the closing "}}".
        assert_eq!(hints[0].position.character, 29);
    }

    #[test]
    fn variables_with_no_set_declaration_get_no_hints() {
        assert!(inlay_hints("GET https://x.com?t={{token}}\n", whole_doc()).is_empty());
    }

    #[test]
    fn only_lines_inside_the_requested_range_are_hinted() {
        let content = "\
# @set token = $.token
GET https://x.com?a={{token}}
GET https://x.com?b={{token}}
";
        let range = Range {
            start: Position {
                line: 2,
                character: 0,
            },
            end: Position {
                line: 2,
                character: 0,
            },
        };
        let hints = inlay_hints(content, range);
        assert_eq!(hints.len(), 1);
        assert_eq!(hints[0].position.line, 2);
    }

    #[test]
    fn several_variables_on_one_line_each_get_their_own_hint() {
        let content = "\
# @set a = $.a
# @set b = $.b
GET https://x.com?p={{a}}&q={{b}}
";
        let hints = inlay_hints(content, whole_doc());
        assert_eq!(hints.len(), 2);
        assert_eq!(label(&hints[0]), "= $.a");
        assert_eq!(label(&hints[1]), "= $.b");
        assert!(hints[0].position.character < hints[1].position.character);
    }

    #[test]
    fn an_unterminated_variable_does_not_panic_or_hint() {
        let content = "# @set token = $.token\nGET https://x.com?t={{token\n";
        assert!(inlay_hints(content, whole_doc()).is_empty());
    }

    #[test]
    fn a_set_declaration_without_an_equals_sign_is_ignored() {
        assert!(
            inlay_hints("# @set token\nGET https://x.com?t={{token}}\n", whole_doc()).is_empty()
        );
    }

    // Hint positions are LSP `Position.character` values, i.e. UTF-16 code
    // units. Emitting the raw byte offset drifted the hint to the right by one
    // column per extra UTF-8 byte earlier in the line.
    #[test]
    fn hint_positions_are_utf16_offsets_not_byte_offsets() {
        let line = "GET https://x.com/日本語?t={{token}}";
        let content = format!("# @set token = $.token\n{line}\n");
        let hints = inlay_hints(&content, whole_doc());
        assert_eq!(hints.len(), 1);
        let expected = line.chars().map(char::len_utf16).sum::<usize>() as u32;
        assert_eq!(
            hints[0].position.character, expected,
            "hint must sit at the end of the line in UTF-16 units, not bytes",
        );
    }

    #[test]
    fn an_empty_document_produces_no_hints() {
        assert!(inlay_hints("", whole_doc()).is_empty());
    }
}
