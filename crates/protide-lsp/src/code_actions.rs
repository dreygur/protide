use crate::semantic_tokens::try_request_line;
use tower_lsp::lsp_types::*;

pub fn code_actions(content: &str, uri: &Url, range: Range) -> Vec<CodeActionOrCommand> {
    let line_num = range.start.line as usize;
    let line = content.lines().nth(line_num).unwrap_or("");
    let trimmed = line.trim_start();

    if try_request_line(trimmed).is_none() {
        return vec![];
    }

    let block_start = block_start_for(content, line_num);
    let has_name = content
        .lines()
        .skip(block_start)
        .take(line_num - block_start)
        .any(|l| l.trim_start().starts_with("# @name"));
    let has_desc = content
        .lines()
        .skip(block_start)
        .take(line_num - block_start)
        .any(|l| l.trim_start().starts_with("# @description"));

    let mut actions = Vec::new();

    if !has_name {
        let method = trimmed
            .split_whitespace()
            .next()
            .unwrap_or("request")
            .to_lowercase();
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
    content
        .lines()
        .enumerate()
        .take(request_line)
        .filter(|(_, l)| l.trim_start().starts_with("###"))
        .map(|(i, _)| i + 1)
        .last()
        .unwrap_or(0)
}

fn make_insert_action(uri: &Url, line: u32, text: String, title: &str) -> CodeActionOrCommand {
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
        edit: Some(WorkspaceEdit {
            changes: Some(changes),
            ..Default::default()
        }),
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url() -> Url {
        Url::parse("file:///tmp/protide-test.http").unwrap()
    }

    fn at(line: u32) -> Range {
        Range {
            start: Position { line, character: 0 },
            end: Position { line, character: 0 },
        }
    }

    fn titles(actions: &[CodeActionOrCommand]) -> Vec<String> {
        actions
            .iter()
            .map(|a| match a {
                CodeActionOrCommand::CodeAction(a) => a.title.clone(),
                CodeActionOrCommand::Command(c) => c.title.clone(),
            })
            .collect()
    }

    #[test]
    fn a_bare_request_line_offers_both_annotations() {
        let actions = code_actions("GET https://example.com\n", &url(), at(0));
        assert_eq!(
            titles(&actions),
            vec!["Add @name annotation", "Add @description annotation"]
        );
    }

    #[test]
    fn the_name_action_seeds_the_method_and_inserts_above_the_request() {
        let actions = code_actions("POST https://example.com\n", &url(), at(0));
        let CodeActionOrCommand::CodeAction(action) = &actions[0] else {
            panic!("expected a code action");
        };
        let edits = &action.edit.as_ref().unwrap().changes.as_ref().unwrap()[&url()];
        assert_eq!(edits[0].new_text, "# @name post\n");
        assert_eq!(edits[0].range.start.line, 0);
        assert_eq!(
            edits[0].range.start, edits[0].range.end,
            "must be an insert"
        );
    }

    #[test]
    fn an_existing_name_annotation_suppresses_the_name_action() {
        let content = "# @name Greeting\nGET https://example.com\n";
        assert_eq!(
            titles(&code_actions(content, &url(), at(1))),
            vec!["Add @description annotation"]
        );
    }

    #[test]
    fn a_fully_annotated_request_offers_nothing() {
        let content = "# @name Greeting\n# @description Says hi\nGET https://example.com\n";
        assert!(code_actions(content, &url(), at(2)).is_empty());
    }

    #[test]
    fn annotations_belonging_to_an_earlier_block_do_not_count() {
        let content = "\
# @name Greeting
GET https://example.com/a

### Second
GET https://example.com/b
";
        assert_eq!(
            titles(&code_actions(content, &url(), at(4))),
            vec!["Add @name annotation", "Add @description annotation"],
            "the previous block's @name must not satisfy this block",
        );
    }

    #[test]
    fn a_non_request_line_offers_nothing() {
        let content = "# @name Greeting\nGET https://example.com\n";
        assert!(code_actions(content, &url(), at(0)).is_empty());
    }

    #[test]
    fn a_range_past_the_end_of_the_document_is_empty_not_a_panic() {
        assert!(code_actions("GET https://example.com\n", &url(), at(9_999)).is_empty());
        assert!(code_actions("", &url(), at(0)).is_empty());
    }
}
