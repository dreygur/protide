//! Requests located in their source text, for callers that show or edit one
//! request of a multi request file.

use crate::ast::Request;
use crate::error::ParseError;

/// A parsed request plus the title on the `###` line that opens its block.
#[derive(Debug, Clone)]
pub struct RequestBlock {
    pub request: Request,
    pub title: Option<String>,
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

/// Parse `content` and pair every request with its block title. A request
/// only owns the title of a `###` line it is the first request after.
pub fn parse_blocks(content: &str) -> Result<Vec<RequestBlock>, ParseError> {
    let requests = crate::parse(content)?;
    let lines: Vec<&str> = content.lines().collect();
    let mut prev_method_line = None;
    let mut blocks = Vec::with_capacity(requests.len());
    for request in requests {
        let method_line = request.line - 1;
        let separator = lines[..method_line].iter().rposition(|l| is_separator(l));
        let first_in_block = prev_method_line.is_none_or(|p| separator.is_some_and(|s| s > p));
        let title = separator.filter(|_| first_in_block).and_then(|s| {
            let text = lines[s].trim().trim_start_matches('#').trim();
            (!text.is_empty()).then(|| text.to_string())
        });
        prev_method_line = Some(method_line);
        blocks.push(RequestBlock { request, title });
    }
    Ok(blocks)
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
