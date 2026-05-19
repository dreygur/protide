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
        let Some(rest) = line.trim_start().strip_prefix("# @set") else { continue };
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

        let Some(close) = search.find("}}") else { break };
        let var_name = &search[..close];

        if let Some(expr) = set_vars.get(var_name) {
            let char_pos = (offset + close + 2) as u32;
            hints.push(InlayHint {
                position: Position { line: line_num, character: char_pos },
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
