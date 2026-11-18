//! Requests located in their source text, for callers that show or edit one
//! request of a multi request file.

use crate::ast::Request;
use crate::error::ParseError;
use std::ops::Range;

/// A parsed request plus where it sits in the source text.
#[derive(Debug, Clone)]
pub struct RequestBlock {
    pub request: Request,
    /// Title on the `###` line that opens the block, if this request is the
    /// first one after it.
    pub title: Option<String>,
    /// Byte range the request owns: everything after its `###` line (or from
    /// its request line when it shares a block) up to the next `###` line or
    /// the next request line. The `###` line itself is never included.
    pub span: Range<usize>,
}

impl RequestBlock {
    /// Display name: `@name`, else the `###` title, else method and URL.
    pub fn label(&self) -> String {
        let req = &self.request;
        req.meta
            .name
            .clone()
            .or_else(|| self.title.clone())
            .unwrap_or_else(|| format!("{} {}", req.method.as_str(), req.url))
    }
}

fn is_separator(line: &str) -> bool {
    line.trim_start_matches('\u{feff}')
        .trim()
        .starts_with("###")
}

/// Parse `content` and locate every request in it. Ownership follows the
/// parser: annotations between two requests with no `###` between them
/// belong to the earlier one.
pub fn parse_blocks(content: &str) -> Result<Vec<RequestBlock>, ParseError> {
    let requests = crate::parse(content)?;
    let lines: Vec<&str> = content.split_inclusive('\n').collect();
    let mut offsets: Vec<usize> = lines
        .iter()
        .scan(0, |at, l| {
            let start = *at;
            *at += l.len();
            Some(start)
        })
        .collect();
    offsets.push(content.len());
    let method_lines: Vec<usize> = requests.iter().map(|r| r.line - 1).collect();

    let mut blocks = Vec::with_capacity(requests.len());
    for (i, request) in requests.into_iter().enumerate() {
        let method_line = method_lines[i];
        let separator = lines[..method_line].iter().rposition(|l| is_separator(l));
        let first_in_block = i == 0 || separator.is_some_and(|s| s > method_lines[i - 1]);
        let start = if first_in_block {
            separator.map_or(0, |s| s + 1)
        } else {
            method_line
        };
        let next_separator = lines[method_line + 1..]
            .iter()
            .position(|l| is_separator(l))
            .map_or(lines.len(), |p| method_line + 1 + p);
        let end = method_lines
            .get(i + 1)
            .map_or(next_separator, |&n| n.min(next_separator));
        let title = separator.filter(|_| first_in_block).and_then(|s| {
            let text = lines[s].trim().trim_start_matches('#').trim();
            (!text.is_empty()).then(|| text.to_string())
        });
        blocks.push(RequestBlock {
            request,
            title,
            span: offsets[start]..offsets[end],
        });
    }
    Ok(blocks)
}

/// `content` with request `index` rewritten from `request`. Every byte
/// outside that request's span, the `###` title line included, is kept, and
/// so are the blank lines that open and end the span. `None` when the content does
/// not parse or has no request `index`.
pub fn replace_request(content: &str, index: usize, request: &Request) -> Option<String> {
    let span = parse_blocks(content).ok()?.get(index)?.span.clone();
    let old = &content[span.clone()];
    let first = old.len() - old.trim_start().len();
    let lead = &old[..old[..first].rfind('\n').map_or(0, |n| n + 1)];
    let kept = old.trim_end().len();
    let tail = old[kept..].find('\n').map_or("", |n| &old[kept + n + 1..]);
    let mut text = request.to_http().trim_end().to_string() + "\n";
    if content.contains("\r\n") {
        text = text.replace('\n', "\r\n");
    }
    Some(
        [
            &content[..span.start],
            lead,
            &text,
            tail,
            &content[span.end..],
        ]
        .concat(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::HttpMethod;
    use pretty_assertions::assert_eq;

    const USERS: &str = "\
### List users
GET https://x.test/users

### Create user
# @name create-user
POST https://x.test/users
Content-Type: application/json

{\"name\": \"a\"}

###
DELETE https://x.test/users/1
";

    #[test]
    fn every_request_in_a_multi_request_file_is_a_block() {
        let blocks = parse_blocks(USERS).unwrap();
        let labels: Vec<_> = blocks.iter().map(RequestBlock::label).collect();
        assert_eq!(
            labels,
            vec!["List users", "create-user", "DELETE https://x.test/users/1"]
        );
    }

    #[test]
    fn a_name_wins_over_the_title() {
        let blocks = parse_blocks(USERS).unwrap();
        assert_eq!(blocks[1].title.as_deref(), Some("Create user"));
        assert_eq!(blocks[1].label(), "create-user");
    }

    #[test]
    fn a_second_request_without_its_own_separator_has_no_title() {
        let blocks = parse_blocks("### Only\nGET https://a.test\n\nGET https://b.test\n").unwrap();
        assert_eq!(blocks[0].title.as_deref(), Some("Only"));
        assert_eq!(blocks[1].title, None);
    }

    #[test]
    fn a_file_without_separators_has_untitled_blocks() {
        let blocks = parse_blocks("\u{feff}GET https://a.test\n").unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].label(), "GET https://a.test");
    }

    fn edited(content: &str, index: usize, edit: impl Fn(&mut Request)) -> String {
        let mut req = parse_blocks(content).unwrap()[index].request.clone();
        edit(&mut req);
        replace_request(content, index, &req).unwrap()
    }

    const FOUR: &str = "\
# shared notes for this file

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

###
GET {{base}}/me
";

    #[test]
    fn editing_the_middle_request_keeps_every_other_byte() {
        let out = edited(FOUR, 1, |r| r.url = "{{base}}/people".into());
        let before = parse_blocks(FOUR).unwrap();
        let after = parse_blocks(&out).unwrap();
        assert_eq!(after.len(), 4);
        assert_eq!(&out[..before[1].span.start], &FOUR[..before[1].span.start]);
        assert_eq!(
            &out[after[1].span.end..],
            &FOUR[before[1].span.end..],
            "requests after the edited one must be untouched"
        );
        assert_eq!(after[1].request.url, "{{base}}/people");
    }

    #[test]
    fn the_edited_request_keeps_its_title_name_set_and_tests() {
        let out = edited(FOUR, 1, |r| r.method = HttpMethod::Put);
        let block = &parse_blocks(&out).unwrap()[1];
        assert_eq!(block.title.as_deref(), Some("Create user"));
        let req = &block.request;
        assert_eq!(req.method, HttpMethod::Put);
        assert_eq!(req.meta.name.as_deref(), Some("create-user"));
        assert_eq!(req.meta.variable_extractions[0].name, "id");
        assert_eq!(
            req.scripts.tests.as_deref(),
            Some("expect(response.status).toBe(201);\n")
        );
        assert_eq!(req.body.as_deref(), Some("{\"name\": \"a\"}"));
    }

    #[test]
    fn an_unchanged_save_of_a_canonical_block_is_byte_identical() {
        for i in 0..4 {
            assert_eq!(edited(FOUR, i, |_| {}), FOUR, "request {i}");
        }
    }

    #[test]
    fn the_last_request_and_a_single_request_file_are_replaced_in_place() {
        let out = edited(FOUR, 3, |r| r.url = "{{base}}/me2".into());
        assert!(out.ends_with("###\nGET {{base}}/me2\n"), "{out:?}");
        let single = "### One\n# @name one\n\nGET https://a.test\n";
        let out = edited(single, 0, |r| r.url = "https://b.test".into());
        assert_eq!(out, "### One\n# @name one\n\nGET https://b.test\n");
    }

    #[test]
    fn requests_sharing_a_block_are_split_at_their_request_lines() {
        let content = "GET https://a.test\n\nGET https://b.test\n";
        let out = edited(content, 1, |r| r.url = "https://c.test".into());
        assert_eq!(out, "GET https://a.test\n\nGET https://c.test\n");
    }

    #[test]
    fn blank_lines_between_the_title_and_the_request_are_kept() {
        let content = "### A\n\nGET https://a.test\n\n### B\nGET https://b.test\n";
        assert_eq!(edited(content, 0, |_| {}), content);
    }

    #[test]
    fn crlf_files_stay_crlf() {
        let content = "### A\r\nGET https://a.test\r\n\r\n### B\r\nGET https://b.test\r\n";
        let out = edited(content, 0, |r| r.url = "https://z.test".into());
        assert_eq!(
            out,
            "### A\r\nGET https://z.test\r\n\r\n### B\r\nGET https://b.test\r\n"
        );
    }

    #[test]
    fn a_missing_index_or_unparseable_file_replaces_nothing() {
        let req = Request::new(HttpMethod::Get, "https://x.test".into());
        assert!(replace_request(FOUR, 4, &req).is_none());
        assert!(replace_request("POST\n\n", 0, &req).is_none());
    }
}
