use tower_lsp::lsp_types::*;
use crate::semantic_tokens::try_request_line;

pub fn code_actions(content: &str, uri: &Url, range: Range) -> Vec<CodeActionOrCommand> {
    let line_num = range.start.line as usize;
    let line = content.lines().nth(line_num).unwrap_or("");
    let trimmed = line.trim_start();

    if try_request_line(trimmed).is_none() {
        return vec![];
    }

    let block_start = block_start_for(content, line_num);
    let has_name = content.lines()
        .skip(block_start)
        .take(line_num - block_start)
        .any(|l| l.trim_start().starts_with("# @name"));
    let has_desc = content.lines()
        .skip(block_start)
        .take(line_num - block_start)
        .any(|l| l.trim_start().starts_with("# @description"));

    let mut actions = Vec::new();

    if !has_name {
        let method = trimmed.split_whitespace().next().unwrap_or("request").to_lowercase();
        let insert_line = line_num as u32;
        actions.push(make_insert_action(
            uri,
            insert_line,
            format!("# @name {method}\n"),
            "Add @name annotation",
        ));
    }

    if !has_desc {
        let insert_line = line_num as u32;
        actions.push(make_insert_action(
            uri,
            insert_line,
            "# @description \n".to_string(),
            "Add @description annotation",
        ));
    }

    actions
}

/// Returns the line index where the current request block starts (after `###` or file start).
fn block_start_for(content: &str, request_line: usize) -> usize {
    content.lines()
        .enumerate()
        .take(request_line)
        .filter(|(_, l)| l.trim_start().starts_with("###"))
        .map(|(i, _)| i + 1)
        .last()
        .unwrap_or(0)
}

fn make_insert_action(
    uri: &Url,
    line: u32,
    text: String,
    title: &str,
) -> CodeActionOrCommand {
    let edit = TextEdit {
        range: Range {
            start: Position { line, character: 0 },
            end: Position { line, character: 0 },
        },
        new_text: text,
    };
    let mut changes = std::collections::HashMap::new();
    changes.insert(uri.clone(), vec![edit]);
    CodeActionOrCommand::CodeAction(CodeAction {
        title: title.to_string(),
        kind: Some(CodeActionKind::QUICKFIX),
        edit: Some(WorkspaceEdit { changes: Some(changes), ..Default::default() }),
        ..Default::default()
    })
}
