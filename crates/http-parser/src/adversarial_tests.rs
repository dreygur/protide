//! Adversarial corpus for the .http parser.
//!
//! `.http` files are a trust boundary: they are hand-written, but they also
//! arrive from third parties through the Postman/Bruno/OpenAPI importers and
//! through collaboration sync. The property under test throughout this module
//! is therefore blunt: **no input may panic, and no input may fail to
//! terminate.** Where an input has one obviously-correct parse, the resulting
//! AST is asserted too; where the format is genuinely ambiguous, the chosen
//! interpretation is pinned with a comment explaining why it is the right one.

use crate::lexer::{Lexer, Token};
use crate::{HttpMethod, ParseError, Protocol, Request, parse};
use pretty_assertions::assert_eq;

/// Structural fingerprint of a parse result. `Request` has no `PartialEq`, so
/// equivalence between two inputs is asserted on the pretty-printed AST, which
/// also gives a readable diff when it fails.
fn shape(requests: &[Request]) -> String {
    format!("{requests:#?}")
}

fn ok(content: &str) -> Vec<Request> {
    parse(content).unwrap_or_else(|e| panic!("expected {content:?} to parse, got error: {e}"))
}

// ---------------------------------------------------------------------------
// Degenerate and empty documents
// ---------------------------------------------------------------------------

#[test]
fn degenerate_documents_produce_no_requests() {
    for input in [
        "",
        " ",
        "\t",
        "\n",
        "\n\n\n",
        "   \t  \n \t \n",
        "\r\n\r\n",
        "#",
        "#\n",
        "###",
        "###\n",
        "####################",
        "###\n###\n###\n",
        "#\n#\n#\n",
        "# a plain comment",
        "### a titled separator with no request",
        "\u{feff}",
        "\u{feff}\n",
    ] {
        let requests = parse(input)
            .unwrap_or_else(|e| panic!("{input:?} should parse as an empty document, got: {e}"));
        assert!(
            requests.is_empty(),
            "{input:?} should yield no requests, got {}",
            requests.len()
        );
    }
}

#[test]
fn a_separator_title_is_not_mistaken_for_a_request() {
    // `### GET something` is a separator with a title, not a request line.
    let requests = ok("### GET the users\n");
    assert!(requests.is_empty());
}

// ---------------------------------------------------------------------------
// Truncated constructs
// ---------------------------------------------------------------------------

#[test]
fn a_request_line_without_a_url_is_an_error_not_a_panic() {
    for input in [
        "GET",
        "GET\n",
        "GET   \n",
        "POST\nAccept: */*\n",
        "WS\n\n",
        "GET\n\n\n",
        "### x\n# @name a\nDELETE\n",
    ] {
        assert!(
            matches!(parse(input), Err(ParseError::MissingUrl { .. })),
            "{input:?} should report a missing URL, got {:?}",
            parse(input)
        );
    }
}

#[test]
fn a_separator_where_the_url_belongs_is_a_clean_error() {
    let err = parse("GET\n###\nGET https://api.test/ok\n").unwrap_err();
    assert!(
        matches!(err, ParseError::UnexpectedToken { .. }),
        "got {err:?}"
    );
}

#[test]
fn a_header_without_a_colon_is_treated_as_body_content() {
    // There is no third option: the line is not a header and not a URL, so the
    // only non-lossy thing to do is keep it as body text.
    let requests = ok("GET https://api.test/x\nNotAHeaderAtAll\n");
    assert_eq!(requests.len(), 1);
    assert!(requests[0].headers.is_empty());
    assert_eq!(requests[0].body.as_deref(), Some("NotAHeaderAtAll"));
}

#[test]
fn a_header_with_an_empty_value_is_preserved() {
    let requests = ok("GET https://api.test/x\nX-Empty:\nX-Spaces:    \n");
    assert_eq!(requests[0].headers.len(), 2);
    assert_eq!(requests[0].get_header("X-Empty"), Some(""));
    assert_eq!(requests[0].get_header("X-Spaces"), Some(""));
}

#[test]
fn a_colon_with_no_key_is_not_a_header() {
    let requests = ok("GET https://api.test/x\n: orphaned value\n");
    assert!(requests[0].headers.is_empty());
    assert_eq!(requests[0].body.as_deref(), Some(": orphaned value"));
}

#[test]
fn an_unterminated_body_is_captured_verbatim() {
    let requests = ok("POST https://api.test/x\nContent-Type: application/json\n\n{\"a\": [1, 2");
    assert_eq!(requests[0].body.as_deref(), Some("{\"a\": [1, 2"));
    assert!(requests[0].has_body());
}

#[test]
fn truncated_annotations_are_ignored_rather_than_failing() {
    // An annotation keyword with no value must leave the field unset instead
    // of erroring - half-typed files are the normal state of an open editor.
    let requests = ok(
        "# @name\n# @description\n# @proto\n# @depends\n# @protocol\n# @\n# @unknown-annotation\nGET https://api.test/x\n",
    );
    assert_eq!(requests.len(), 1);
    let meta = &requests[0].meta;
    assert_eq!(meta.name, None);
    assert_eq!(meta.description, None);
    assert_eq!(meta.proto_path, None);
    assert_eq!(meta.protocol, None);
    assert!(meta.depends.is_empty());
}

#[test]
fn set_without_an_expression_records_no_extraction() {
    for line in ["# @set", "# @set token", "# @set token $.token", "# @set  "] {
        let requests = ok(&format!("GET https://api.test/x\n\n{line}\n"));
        assert!(
            requests[0].meta.variable_extractions.is_empty(),
            "{line:?} has no `=` and so defines no variable"
        );
    }
}

#[test]
fn set_with_an_empty_expression_still_records_the_name() {
    // The extraction is kept so the UI can show the half-written rule; the
    // empty JSONPath fails at extraction time rather than at parse time.
    let requests = ok("GET https://api.test/x\n\n# @set token =\n");
    let extractions = &requests[0].meta.variable_extractions;
    assert_eq!(extractions.len(), 1);
    assert_eq!(extractions[0].name, "token");
    assert_eq!(extractions[0].expression, "");
}

#[test]
fn an_unknown_protocol_is_reported_with_its_line() {
    let err = parse("### x\n# @name a\n# @protocol quantum\nGET https://api.test/x\n").unwrap_err();
    match err {
        ParseError::InvalidProtocol { line, protocol } => {
            assert_eq!(line, 3, "the @protocol annotation is on line 3");
            assert_eq!(protocol, "quantum");
        }
        other => panic!("expected InvalidProtocol, got {other:?}"),
    }
}

#[test]
fn every_known_protocol_alias_resolves() {
    for (alias, expected) in [
        ("http", Protocol::Http),
        ("REST", Protocol::Http),
        ("GraphQL", Protocol::GraphQL),
        ("gql", Protocol::GraphQL),
        ("websocket", Protocol::WebSocket),
        ("WS", Protocol::WebSocket),
        ("grpc", Protocol::Grpc),
        ("socket.io", Protocol::SocketIO),
        ("SocketIO", Protocol::SocketIO),
        ("tRPC", Protocol::Trpc),
    ] {
        let requests = ok(&format!("# @protocol {alias}\nGET https://api.test/x\n"));
        assert_eq!(
            requests[0].protocol(),
            expected,
            "@protocol {alias} should resolve to {expected:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Line endings
// ---------------------------------------------------------------------------

/// A document exercising every construct, used for line-ending equivalence.
const SAMPLE: &str = "### Create User\n\
# @name create-user\n\
# @description Creates a user\n\
\n\
POST https://api.test/users\n\
Content-Type: application/json\n\
# X-Disabled: nope\n\
\n\
{\"name\": \"John\"}\n\
\n\
# @set id = $.id\n\
\n\
# @tests\n\
expect(response.status).toBe(200);\n\
\n\
### Get Users\n\
GET https://api.test/users\n";

#[test]
fn crlf_parses_identically_to_lf() {
    let lf = ok(SAMPLE);
    let crlf = ok(&SAMPLE.replace('\n', "\r\n"));
    assert_eq!(
        shape(&lf),
        shape(&crlf),
        "a CRLF file must parse exactly like its LF twin"
    );
    assert_eq!(lf.len(), 2);
}

#[test]
fn mixed_line_endings_parse_identically_to_pure_lf() {
    // Every other terminator is a CRLF - the state a file reaches after being
    // edited on two platforms, or after a sloppy merge.
    let mixed: String = SAMPLE
        .split_inclusive('\n')
        .enumerate()
        .map(|(i, l)| {
            if i % 2 == 0 {
                l.to_string()
            } else {
                l.replace('\n', "\r\n")
            }
        })
        .collect();
    assert_eq!(shape(&ok(SAMPLE)), shape(&ok(&mixed)));
}

#[test]
fn a_missing_final_newline_changes_nothing() {
    let with = ok(SAMPLE);
    let without = ok(SAMPLE.trim_end_matches('\n'));
    assert_eq!(shape(&with), shape(&without));
}

#[test]
fn lone_carriage_returns_terminate_without_panicking() {
    // Classic-Mac CR-only files are not supported: `str::lines` splits on \n
    // and on \r\n only, so a CR-only document is one logical line. That is the
    // deliberate choice - a bare \r inside a request body must survive
    // verbatim - so the guarantee here is only that it parses and terminates.
    let cr_only = "GET https://api.test/x\rAccept: */*\r";
    let requests = ok(cr_only);
    assert_eq!(requests.len(), 1);
    assert!(
        requests[0].url.contains('\r'),
        "a CR-only file collapses into a single line"
    );

    // A bare \r in the middle of an otherwise LF document must not disturb it.
    let requests = ok("GET https://api.test/x\nX-Odd: a\rb\n");
    assert_eq!(requests[0].get_header("X-Odd"), Some("a\rb"));
}

// ---------------------------------------------------------------------------
// Byte-order mark
// ---------------------------------------------------------------------------

#[test]
fn a_leading_bom_is_ignored() {
    // Regression: a BOM is not whitespace, so before it was stripped it glued
    // itself to the method keyword and made the whole file unparseable.
    let plain = ok(SAMPLE);
    let bommed = ok(&format!("\u{feff}{SAMPLE}"));
    assert_eq!(shape(&plain), shape(&bommed));
}

#[test]
fn a_bom_before_a_bare_request_line_is_ignored() {
    let requests = ok("\u{feff}GET https://api.test/users\n");
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, HttpMethod::Get);
    assert_eq!(requests[0].url, "https://api.test/users");
    assert_eq!(requests[0].line, 1, "the BOM must not shift line numbers");
}

#[test]
fn a_bom_in_the_middle_of_a_file_stays_in_the_text() {
    // Only a leading BOM is an encoding artefact; anywhere else it is content.
    let requests = ok("GET https://api.test/x\nX-Mark: a\u{feff}b\n");
    assert_eq!(requests[0].get_header("X-Mark"), Some("a\u{feff}b"));
}

// ---------------------------------------------------------------------------
// Multi-byte UTF-8
//
// Every byte offset in the lexer comes from `find(':')` or `find('=')` and is
// then incremented by one. Those are one-byte ASCII delimiters, so the +1 is
// always on a char boundary - these tests pin that down, because getting it
// wrong is a slice panic on the very first non-ASCII file a user opens.
// ---------------------------------------------------------------------------

#[test]
fn multibyte_urls_survive_intact() {
    let url = "https://例え.テスト/パス/セグメント?q=🚀&flag=🇯🇵#フラグメント";
    let requests = ok(&format!("GET {url}\n"));
    assert_eq!(requests[0].url, url);
}

#[test]
fn multibyte_header_names_and_values_round_trip() {
    // `is_alphanumeric` is Unicode-aware, so a Cyrillic or CJK key is accepted
    // as a header name here and rejected later by the HTTP client. Keeping it
    // in the AST is what lets the editor show the user what they typed.
    let requests = ok("GET https://api.test/x\n\
X-Emoji: 🚀 → ✅\n\
Заголовок: значение\n\
日本語: 値\n\
X-Ünïcödé: café ☕\n");
    assert_eq!(requests[0].headers.len(), 4);
    assert_eq!(requests[0].get_header("X-Emoji"), Some("🚀 → ✅"));
    assert_eq!(requests[0].get_header("Заголовок"), Some("значение"));
    assert_eq!(requests[0].get_header("日本語"), Some("値"));
    assert_eq!(requests[0].get_header("X-Ünïcödé"), Some("café ☕"));
}

#[test]
fn a_non_alphanumeric_key_is_not_a_header() {
    // Emoji and a full-width colon both fail the key test, so these lines are
    // body text rather than silently-corrupted headers.
    for line in ["🚀: launch", "名前：値", "a b: c"] {
        let requests = ok(&format!("GET https://api.test/x\n{line}\n"));
        assert!(
            requests[0].headers.is_empty(),
            "{line:?} should not be a header"
        );
        assert_eq!(requests[0].body.as_deref(), Some(line));
    }
}

#[test]
fn multibyte_annotation_values_round_trip() {
    let requests = ok("# @name 🚀 デプロイ テスト\n\
# @description Ünïcödé — \"smart quotes\" … ✅\n\
# @depends 前提-1, 前提-2\n\
GET https://api.test/x\n");
    let meta = &requests[0].meta;
    assert_eq!(meta.name.as_deref(), Some("🚀 デプロイ テスト"));
    assert_eq!(
        meta.description.as_deref(),
        Some("Ünïcödé — \"smart quotes\" … ✅")
    );
    assert_eq!(
        meta.depends,
        vec!["前提-1".to_string(), "前提-2".to_string()]
    );
}

#[test]
fn multibyte_set_expressions_split_on_the_right_byte() {
    let requests = ok("GET https://api.test/x\n\n# @set トークン🔑 = $.データ.トークン\n");
    let extractions = &requests[0].meta.variable_extractions;
    assert_eq!(extractions.len(), 1);
    assert_eq!(extractions[0].name, "トークン🔑");
    assert_eq!(extractions[0].expression, "$.データ.トークン");
}

#[test]
fn multibyte_bodies_are_preserved_exactly() {
    let body = "{\"emoji\": \"👨‍👩‍👧‍👦\", \"jp\": \"日本語\", \"rtl\": \"مرحبا\", \"combining\": \"a\u{0301}e\u{0308}\"}";
    let requests = ok(&format!(
        "POST https://api.test/x\nContent-Type: application/json\n\n{body}\n"
    ));
    assert_eq!(requests[0].body.as_deref(), Some(body));
}

#[test]
fn a_lone_multibyte_line_does_not_derail_the_parse() {
    for line in ["🚀", "日本語", "\u{200b}", "\u{0301}", "\u{fffd}"] {
        let requests = ok(&format!("GET https://api.test/x\n\n{line}\n"));
        assert_eq!(requests[0].body.as_deref(), Some(line));
    }
}

// ---------------------------------------------------------------------------
// Line and column reporting
// ---------------------------------------------------------------------------

#[test]
fn line_numbers_are_unaffected_by_multibyte_content() {
    let content = "### 見出し 🚀\n\
# @name テスト\n\
\n\
# コメント\n\
GET https://api.test/x\n\
X-Ünïcödé: ☕\n\
\n\
### 二番目 🇯🇵\n\
\n\
POST https://api.test/y\n";
    let requests = ok(content);
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].line, 5, "GET is on line 5");
    assert_eq!(requests[1].line, 10, "POST is on line 10");
}

#[test]
fn line_numbers_survive_crlf() {
    let content =
        "### a\n# @name x\n\nGET https://api.test/x\n\n### b\n\n\n\nPOST https://api.test/y\n";
    let lf = ok(content);
    let crlf = ok(&content.replace('\n', "\r\n"));
    assert_eq!(lf[0].line, 4);
    assert_eq!(lf[1].line, 10);
    assert_eq!(shape(&lf), shape(&crlf));
}

#[test]
fn error_line_numbers_survive_multibyte_and_crlf() {
    let content = "### 🚀\n# @name テスト\n# @protocol 未知のプロトコル\nGET https://api.test/x\n";
    for src in [content.to_string(), content.replace('\n', "\r\n")] {
        match parse(&src).unwrap_err() {
            ParseError::InvalidProtocol { line, protocol } => {
                assert_eq!(line, 3);
                assert_eq!(protocol, "未知のプロトコル");
            }
            other => panic!("expected InvalidProtocol, got {other:?}"),
        }
    }
}

#[test]
fn the_lexer_tags_every_token_with_its_source_line() {
    let content =
        "### 見出し\r\n# @name テスト\r\n\r\nGET https://api.test/🚀\r\nX-Ünïcödé: ☕\r\n";
    let spans: Vec<_> = Lexer::new(content).collect();
    assert_eq!(
        spans.iter().map(|s| s.line).collect::<Vec<_>>(),
        vec![1, 2, 3, 4, 4, 5],
        "the URL shares the method's line; everything else is sequential"
    );
    assert!(matches!(&spans[4].token, Token::Url(u) if u == "https://api.test/🚀"));
    assert!(matches!(&spans[5].token, Token::Header(k, v) if k == "X-Ünïcödé" && v == "☕"));
}

#[test]
fn peeking_never_consumes_a_token() {
    let mut lexer = Lexer::new("GET https://api.test/x\nAccept: */*\n");
    let peeked = lexer.peek();
    assert_eq!(peeked, lexer.peek(), "peek must be idempotent");
    assert_eq!(peeked, lexer.next_token(), "peek must match the next read");
    assert_eq!(Lexer::new("").peek().token, Token::Eof);
}

// ---------------------------------------------------------------------------
// Ambiguity: `#` comment vs `# @annotation` vs `###` separator vs disabled header
// ---------------------------------------------------------------------------

#[test]
fn a_hash_count_of_three_or_more_is_always_a_separator() {
    // 1-2 hashes introduce a comment/annotation, 3+ introduce a separator.
    // The boundary matters: `### @name x` is a separator titled "@name x".
    let requests = ok("### @name ignored\nGET https://api.test/x\n");
    assert_eq!(requests[0].meta.name, None);

    let requests = ok("#### @name ignored\nGET https://api.test/x\n");
    assert_eq!(requests[0].meta.name, None);
}

#[test]
fn annotations_are_recognised_regardless_of_hash_spacing() {
    for prefix in ["# @", "#@", "#   @", "## @", "##@"] {
        let requests = ok(&format!("{prefix}name tagged\nGET https://api.test/x\n"));
        assert_eq!(
            requests[0].meta.name.as_deref(),
            Some("tagged"),
            "{prefix:?} should introduce an annotation"
        );
    }
}

#[test]
fn an_at_sign_that_does_not_open_the_comment_is_not_an_annotation() {
    let requests = ok("# see @name below, mail a@b.test\nGET https://api.test/x\n");
    assert_eq!(requests[0].meta.name, None);
}

#[test]
fn a_colon_shaped_comment_in_the_header_block_is_a_disabled_header() {
    // Deliberate, and load-bearing: it is how a header toggled off in the UI
    // round-trips through save/load. The cost is that a `Key: value`-shaped
    // note written among the headers is captured as a disabled header, which
    // is why only the header block is scanned this way.
    let requests =
        ok("GET https://api.test/x\nAccept: */*\n# TODO: revisit\n# Ключ: значение\n\n{\"a\":1}\n");
    let headers = &requests[0].headers;
    assert_eq!(headers.len(), 3);
    assert_eq!(
        (headers[1].key.as_str(), headers[1].enabled),
        ("TODO", false)
    );
    assert_eq!(headers[2].value, "значение");
    assert!(!headers[2].enabled);
}

#[test]
fn a_colon_shaped_comment_outside_the_header_block_stays_a_comment() {
    let requests = ok("### x\n\
# TODO: before the request\n\
GET https://api.test/x\n\
Accept: */*\n\
\n\
{\"a\":1}\n\
\n\
# NOTE: after the body\n");
    assert_eq!(requests[0].headers.len(), 1);
    assert_eq!(requests[0].body.as_deref(), Some("{\"a\":1}"));
}

#[test]
fn a_comment_that_cannot_be_a_header_closes_the_header_block() {
    let requests = ok("GET https://api.test/x\nAccept: */*\n# 🚀: not a valid key\n");
    assert_eq!(requests[0].headers.len(), 1);
    assert!(requests[0].body.is_none());
}

#[test]
fn a_header_shaped_line_after_the_body_started_stays_in_the_body() {
    let requests = ok(
        "POST https://api.test/x\nContent-Type: application/json\n\n{\"a\": 1}\nX-Not-A-Header: still body\n",
    );
    assert_eq!(requests[0].headers.len(), 1);
    assert_eq!(
        requests[0].body.as_deref(),
        Some("{\"a\": 1}\nX-Not-A-Header: still body")
    );
}

#[test]
fn a_hash_line_always_closes_the_body() {
    // Sharp edge, documented rather than fixed: `#` is structural everywhere,
    // so a body that legitimately contains a `#`-prefixed line (a shell
    // script, a YAML document) is cut short there. The follow-on text then
    // stands where a request line is expected, which is a clean error rather
    // than a panic or a hang - which is the property this test defends.
    let err = parse("POST https://api.test/x\nContent-Type: text/plain\n\n#!/bin/sh\necho hi\n")
        .unwrap_err();
    assert!(
        matches!(err, ParseError::UnexpectedToken { .. }),
        "got {err:?}"
    );
}

/// REGRESSION: a scheme-prefixed URL on a line of its own used to match the
/// header rule first - `find(':')` split `https://api.test/x` into key `https`
/// and value `//api.test/x` - so the lexer's standalone-URL branch was
/// unreachable and `Parser::parse_request`'s `Token::Url` arm was dead code.
/// Inside a header block this was silent: a bare URL line became a header named
/// after its scheme and would have gone out on the wire.
///
/// FIXED: `URL_SCHEMES` is tested before the header rule. Only the scheme test
/// moved - the `{{var}}` URL form still has to be decided *after* the header
/// rule, or `Authorization: Bearer {{token}}` would lex as a URL.
#[test]
fn a_url_on_its_own_line_is_recognised_as_the_url() {
    // The request-line spelling the parser clearly intends to accept:
    // `Parser::parse_request` has a `Token::Url` arm reached only from here.
    assert_eq!(ok("GET\nhttps://api.test/x\n")[0].url, "https://api.test/x");
    assert_eq!(ok("GET\nws://api.test/sock\n")[0].url, "ws://api.test/sock");

    // The silent-corruption case: a stray URL line among the headers used to
    // become a header named `https` and go out on the wire. It is now a loud
    // parse error - malformed input the user can see and fix, rather than a
    // request that quietly carries a header they never wrote.
    let err = parse("GET https://api.test/x\nAccept: */*\nhttps://api.test/y\n")
        .expect_err("a stray URL line among headers must not parse silently");
    assert!(
        matches!(err, ParseError::UnexpectedToken { line: 3, .. }),
        "{err:?}"
    );
}

/// A header value containing a variable must keep lexing as a header. This is
/// what stops the fix above from being "test the URL rule first" - that rule
/// also matches any line containing `{{`, so hoisting it wholesale would turn
/// every variable-bearing header into a URL.
#[test]
fn a_header_whose_value_holds_a_variable_is_still_a_header() {
    let requests = ok("GET https://api.test/x\nAuthorization: Bearer {{token}}\n");
    assert_eq!(
        requests[0].get_header("Authorization"),
        Some("Bearer {{token}}")
    );
}

#[test]
#[ignore = "DEFECT: a `{{var}}`-prefixed URL on its own line is still not \
recognised. The URL rule excludes anything starting with `{` to keep JSON \
bodies out, and `{{base_url}}/users` trips that guard, so it becomes body \
text. Unlike the scheme case this one is not silent - it is a clean parse \
error - and the obvious repair (admitting `{{`, which is never valid JSON) \
would reclassify a raw body that is just a variable, e.g. a request whose \
entire body is `{{payload}}`. That is a live format, so the trade needs a \
decision rather than a patch."]
fn a_variable_prefixed_url_on_its_own_line_is_recognised() {
    assert_eq!(ok("GET\n{{base_url}}/users\n")[0].url, "{{base_url}}/users");
}

#[test]
fn a_variable_prefixed_url_on_its_own_line_fails_loudly_rather_than_dangerously() {
    // Companion to the ignored test above: pin the damage while it stands. It
    // must be a clean error, never a panic and never a silently wrong request.
    assert!(matches!(
        parse("GET\n{{base_url}}/users\n"),
        Err(ParseError::UnexpectedToken { .. })
    ));
}

// ---------------------------------------------------------------------------
// Script blocks
// ---------------------------------------------------------------------------

#[test]
fn an_empty_script_block_at_eof_yields_an_empty_script() {
    let requests = ok("GET https://api.test/x\n\n# @tests\n");
    assert_eq!(requests[0].scripts.tests.as_deref(), Some(""));
}

#[test]
fn adjacent_script_blocks_do_not_bleed_into_each_other() {
    let requests = ok("GET https://api.test/x\n\
\n\
# @pre-script\n\
const a = 1;\n\
# @post-script\n\
const b = 2;\n\
# @tests\n\
expect(a).toBe(1);\n\
###\n\
GET https://api.test/y\n");
    assert_eq!(requests.len(), 2);
    assert_eq!(
        requests[0].scripts.pre_script.as_deref(),
        Some("const a = 1;")
    );
    assert_eq!(
        requests[0].scripts.post_script.as_deref(),
        Some("const b = 2;")
    );
    assert_eq!(
        requests[0].scripts.tests.as_deref(),
        Some("expect(a).toBe(1);")
    );
    assert!(requests[1].scripts.tests.is_none());
}

#[test]
fn a_script_marker_with_no_request_is_an_error_not_a_panic() {
    assert!(parse("# @tests\nexpect(1).toBe(1);\n").is_err());
}

// ---------------------------------------------------------------------------
// Pathological sizes - these assert termination, not speed
// ---------------------------------------------------------------------------

#[test]
fn ten_thousand_headers_terminate() {
    let mut doc = String::from("GET https://api.test/x\n");
    for i in 0..10_000 {
        doc.push_str(&format!("X-H{i}: v{i}\n"));
    }
    let requests = ok(&doc);
    assert_eq!(requests[0].headers.len(), 10_000);
    assert_eq!(requests[0].get_header("X-H9999"), Some("v9999"));
}

#[test]
fn a_quarter_megabyte_single_line_terminates() {
    let url = format!("https://api.test/{}", "a".repeat(200_000));
    let requests = ok(&format!("GET {url}\n"));
    assert_eq!(requests[0].url, url);

    let requests = ok(&format!(
        "GET https://api.test/x\nX-Big: {}\n",
        "b".repeat(200_000)
    ));
    assert_eq!(requests[0].get_header("X-Big").map(str::len), Some(200_000));
}

#[test]
fn fifty_thousand_separators_terminate() {
    assert!(ok(&"###\n".repeat(50_000)).is_empty());
    assert!(ok(&"\n".repeat(50_000)).is_empty());
    assert!(ok(&"# a comment\n".repeat(50_000)).is_empty());
}

#[test]
fn annotations_alternating_with_separators_do_not_exhaust_the_stack() {
    // Regression: a `###` standing where the request line was expected used to
    // restart the request by *recursing*, so this document recursed once per
    // separator and aborted the process with a stack overflow. Parsing runs on
    // a deliberately small stack so a reintroduced recursion is caught here
    // rather than on a user's machine.
    let doc = "# @name a\n###\n".repeat(20_000);
    let parsed = std::thread::Builder::new()
        .stack_size(512 * 1024)
        .spawn(move || parse(&doc).map(|r| r.len()))
        .expect("spawn")
        .join()
        .expect("parsing must not overflow the stack");
    assert_eq!(parsed.expect("should parse"), 0);
}

#[test]
fn a_deeply_nested_body_is_not_parsed_recursively() {
    let body = format!("{}{}", "[".repeat(100_000), "]".repeat(100_000));
    let requests = ok(&format!("POST https://api.test/x\n\n{body}\n"));
    assert_eq!(requests[0].body.as_deref(), Some(body.as_str()));
}

#[test]
fn thousands_of_requests_parse() {
    let requests = ok(&"### r\n# @name n\nGET https://api.test/x\n\n".repeat(5_000));
    assert_eq!(requests.len(), 5_000);
    assert_eq!(requests[4_999].method, HttpMethod::Get);
    assert_eq!(requests[4_999].line, 5_000 * 4 - 1);
}

// ---------------------------------------------------------------------------
// Known defects, documented rather than fixed
// ---------------------------------------------------------------------------

#[test]
#[ignore = "DEFECT: `@set` with a blank name records an extraction named \"\", \
which downstream becomes an environment variable with an empty name. It should \
be dropped the same way a `@set` with no `=` is. Left unfixed because the \
change lives in the lexer's annotation path and the empty name is inert in \
practice (the extraction still runs, it just writes to a nameless variable)."]
fn set_with_a_blank_name_defines_nothing() {
    let requests = ok("GET https://api.test/x\n\n# @set  = $.token\n");
    assert!(requests[0].meta.variable_extractions.is_empty());
}

#[test]
fn set_with_a_blank_name_is_at_least_harmless() {
    // Companion to the ignored test above: pin the property that actually
    // matters until the defect is fixed - it parses, and it does not panic.
    let requests = ok("GET https://api.test/x\n\n# @set  = $.token\n");
    assert_eq!(
        requests[0].meta.variable_extractions[0].expression,
        "$.token"
    );
}

#[test]
#[ignore = "DEFECT: `MissingUrl` reports the line of the token the parser is \
holding, which at end of file is one past the last line - `parse(\"GET\")` \
blames line 2 of a one-line document. protide-lsp turns that straight into a \
diagnostic range, so the squiggle lands outside the file. It should blame the \
request line the URL is missing from. Left unfixed because it also shifts the \
non-EOF case (`GET` then a header line) from the header's line to the request \
line, which is a visible change to existing diagnostics."]
fn a_missing_url_is_blamed_on_the_request_line() {
    for (input, expected) in [("GET", 1), ("### x\nGET\n", 2), ("GET\nAccept: */*\n", 1)] {
        match parse(input).unwrap_err() {
            ParseError::MissingUrl { line } => assert_eq!(line, expected, "for {input:?}"),
            other => panic!("expected MissingUrl, got {other:?}"),
        }
    }
}

#[test]
fn error_lines_never_underflow() {
    // Whatever line an error blames, protide-lsp subtracts one from it; a zero
    // would wrap. Every error must therefore carry a 1-based line.
    for input in [
        "GET",
        "GET\n###\n",
        "# @protocol nope\nGET https://api.test/x\n",
        "\u{feff}GET",
    ] {
        if let Err(e) = parse(input) {
            let line = match e {
                ParseError::UnexpectedToken { line, .. }
                | ParseError::InvalidMethod { line, .. }
                | ParseError::MissingUrl { line }
                | ParseError::InvalidUrl { line, .. }
                | ParseError::InvalidProtocol { line, .. } => line,
            };
            assert!(line >= 1, "{input:?} reported line {line}");
        }
    }
}
