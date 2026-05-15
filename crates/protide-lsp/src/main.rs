use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

struct HttpLsp {
    client: Client,
}

#[tower_lsp::async_trait]
impl LanguageServer for HttpLsp {
    async fn initialize(&self, _params: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec!["@".to_string(), "#".to_string(), "{".to_string()]),
                    ..Default::default()
                }),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(SemanticTokensOptions {
                        legend: SemanticTokensLegend {
                            token_types: vec![
                                SemanticTokenType::KEYWORD,
                                SemanticTokenType::STRING,
                                SemanticTokenType::PROPERTY,
                                SemanticTokenType::PARAMETER,
                                SemanticTokenType::COMMENT,
                                SemanticTokenType::NUMBER,
                            ],
                            token_modifiers: vec![SemanticTokenModifier::READONLY],
                        },
                        full: Some(SemanticTokensFullOptions::Bool(true)),
                        ..Default::default()
                    }),
                ),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "protide-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "protide-lsp initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let pos = params.text_document_position_params.position;
        let uri = &params.text_document_position_params.text_document.uri;

        let content = match std::fs::read_to_string(uri.path()) {
            Ok(c) => c,
            Err(_) => return Ok(None),
        };

        let line = content.lines().nth(pos.line as usize).unwrap_or("");
        let word = word_at(line, pos.character as usize);

        let docs = match word.to_uppercase().as_str() {
            "GET" => Some("**GET** — Retrieve a resource. Safe and idempotent."),
            "POST" => Some("**POST** — Submit data to create or process a resource."),
            "PUT" => Some("**PUT** — Replace a resource entirely. Idempotent."),
            "PATCH" => Some("**PATCH** — Partially update a resource."),
            "DELETE" => Some("**DELETE** — Remove a resource. Idempotent."),
            "HEAD" => Some("**HEAD** — Same as GET but returns only headers."),
            "OPTIONS" => Some("**OPTIONS** — Describe communication options for a resource."),
            "@NAME" | "NAME" if line.trim_start().starts_with("# @") => {
                Some("**@name** — Assigns a name to this request for use in chaining with `@depends`.")
            }
            "@DESCRIPTION" | "DESCRIPTION" if line.trim_start().starts_with("# @") => {
                Some("**@description** — Human-readable description of this request. Included in exported docs.")
            }
            "@PROTOCOL" | "PROTOCOL" if line.trim_start().starts_with("# @") => {
                Some("**@protocol** — Override protocol detection.\nValues: `http`, `graphql`, `websocket`, `grpc`, `trpc`, `socketio`")
            }
            "@SET" | "SET" if line.trim_start().starts_with("# @") => {
                Some("**@set** — Extract a value from the response and store it as a variable.\nSyntax: `# @set varName = $.path.to.value`")
            }
            "@DEPENDS" | "DEPENDS" if line.trim_start().starts_with("# @") => {
                Some("**@depends** — Declare that this request depends on another named request.")
            }
            "@PROTO" | "PROTO" if line.trim_start().starts_with("# @") => {
                Some("**@proto** — Path to the .proto file for gRPC requests.")
            }
            _ => None,
        };

        Ok(docs.map(|d| Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: d.to_string(),
            }),
            range: None,
        }))
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let pos = params.text_document_position.position;
        let uri = &params.text_document_position.text_document.uri;

        let content = match std::fs::read_to_string(uri.path()) {
            Ok(c) => c,
            Err(_) => return Ok(None),
        };
        let line = content.lines().nth(pos.line as usize).unwrap_or("").to_string();
        let before = &line[..line.len().min(pos.character as usize)];

        let items = if before.trim_start().starts_with("# @") {
            annotation_completions()
        } else if is_request_line(before) {
            method_completions()
        } else if before.contains(':') {
            header_value_completions(before)
        } else if !before.contains(' ') && !before.starts_with('#') {
            header_name_completions()
        } else {
            vec![]
        };

        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = &params.text_document.uri;
        let content = match std::fs::read_to_string(uri.path()) {
            Ok(c) => c,
            Err(_) => return Ok(None),
        };
        let tokens = tokenize(&content);
        Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data: tokens,
        })))
    }
}

fn word_at(line: &str, char_idx: usize) -> &str {
    let chars: Vec<char> = line.chars().collect();
    let start = chars[..char_idx.min(chars.len())]
        .iter()
        .rposition(|c| !c.is_alphanumeric() && *c != '_' && *c != '@')
        .map(|i| i + 1)
        .unwrap_or(0);
    let end = chars[char_idx.min(chars.len())..]
        .iter()
        .position(|c| !c.is_alphanumeric() && *c != '_' && *c != '@')
        .map(|i| char_idx + i)
        .unwrap_or(chars.len());
    let byte_start = chars[..start].iter().map(|c| c.len_utf8()).sum::<usize>();
    let byte_end = chars[..end].iter().map(|c| c.len_utf8()).sum::<usize>();
    &line[byte_start..byte_end]
}

fn is_request_line(before: &str) -> bool {
    let upper = before.trim_start().to_uppercase();
    ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"]
        .iter()
        .any(|m| upper.starts_with(m) || m.starts_with(upper.trim()))
}

fn annotation_completions() -> Vec<CompletionItem> {
    [
        ("@name", "Name this request for chaining"),
        ("@description", "Human-readable description"),
        ("@protocol", "Override protocol (http|graphql|websocket|grpc|trpc)"),
        ("@set", "Extract response value to variable: @set var = $.path"),
        ("@depends", "Declare dependency on named request"),
        ("@proto", "Path to .proto file for gRPC"),
    ]
    .iter()
    .map(|(label, detail)| CompletionItem {
        label: label.to_string(),
        kind: Some(CompletionItemKind::KEYWORD),
        detail: Some(detail.to_string()),
        ..Default::default()
    })
    .collect()
}

fn method_completions() -> Vec<CompletionItem> {
    ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"]
        .iter()
        .map(|m| CompletionItem {
            label: m.to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            ..Default::default()
        })
        .collect()
}

fn header_name_completions() -> Vec<CompletionItem> {
    [
        ("Content-Type", "application/json"),
        ("Authorization", "Bearer <token>"),
        ("Accept", "application/json"),
        ("X-Request-ID", ""),
        ("X-API-Key", ""),
        ("Cache-Control", "no-cache"),
    ]
    .iter()
    .map(|(name, value)| CompletionItem {
        label: format!("{}: {}", name, value),
        kind: Some(CompletionItemKind::PROPERTY),
        insert_text: Some(format!("{}: ", name)),
        ..Default::default()
    })
    .collect()
}

fn header_value_completions(line: &str) -> Vec<CompletionItem> {
    if line.to_lowercase().contains("content-type:") {
        return ["application/json", "application/x-www-form-urlencoded", "multipart/form-data", "text/plain"]
            .iter()
            .map(|v| CompletionItem {
                label: v.to_string(),
                kind: Some(CompletionItemKind::VALUE),
                ..Default::default()
            })
            .collect();
    }
    vec![]
}

// Semantic token types indices
const TOK_KEYWORD: u32 = 0;
const TOK_STRING: u32 = 1;
const TOK_PROPERTY: u32 = 2;
const TOK_PARAMETER: u32 = 3;
const TOK_COMMENT: u32 = 4;

fn tokenize(content: &str) -> Vec<SemanticToken> {
    let mut tokens = Vec::new();
    let mut prev_line = 0u32;
    let mut prev_start = 0u32;

    for (line_num, line) in content.lines().enumerate() {
        let ln = line_num as u32;
        let trimmed = line.trim_start();

        let tok = if trimmed.starts_with("###") {
            Some((0u32, line.len() as u32, TOK_KEYWORD))
        } else if trimmed.starts_with("# @") || trimmed.starts_with("# @") {
            Some((0, line.len() as u32, TOK_COMMENT))
        } else if trimmed.starts_with('#') {
            Some((0, line.len() as u32, TOK_COMMENT))
        } else if let Some(rest) = try_request_line(trimmed) {
            let method_end = line.find(' ').unwrap_or(line.len());
            let indent = (line.len() - trimmed.len()) as u32;
            tokens.push(make_token(ln, prev_line, indent, prev_start, method_end as u32, TOK_KEYWORD));
            prev_line = ln;
            prev_start = indent;
            let url_start = indent + method_end as u32 + 1;
            let url_len = rest.trim().len() as u32;
            Some((url_start, url_len, TOK_STRING))
        } else if trimmed.contains(':') && !trimmed.starts_with('{') {
            let colon = line.find(':').unwrap_or(0);
            let indent = (line.len() - trimmed.len()) as u32;
            tokens.push(make_token(ln, prev_line, indent, prev_start, colon as u32, TOK_PROPERTY));
            prev_line = ln;
            prev_start = indent;
            None
        } else {
            None
        };

        if let Some((start, len, tok_type)) = tok {
            tokens.push(make_token(ln, prev_line, start, prev_start, len, tok_type));
            prev_line = ln;
            prev_start = start;
        }

        // Highlight {{variables}} within the line
        let mut search = line;
        let mut offset = 0usize;
        while let Some(open) = search.find("{{") {
            if let Some(close) = search[open + 2..].find("}}") {
                let var_start = (offset + open) as u32;
                let var_len = (close + 4) as u32;
                tokens.push(make_token(ln, prev_line, var_start, prev_start, var_len, TOK_PARAMETER));
                prev_line = ln;
                prev_start = var_start;
                let skip = open + close + 4;
                offset += skip;
                search = &search[skip.min(search.len())..];
            } else {
                break;
            }
        }
    }

    tokens
}

fn try_request_line(s: &str) -> Option<&str> {
    let methods = ["GET ", "POST ", "PUT ", "PATCH ", "DELETE ", "HEAD ", "OPTIONS ", "WEBSOCKET ", "GRPC "];
    for m in &methods {
        if s.starts_with(m) {
            return Some(&s[m.len()..]);
        }
    }
    None
}

fn make_token(line: u32, prev_line: u32, start: u32, prev_start: u32, len: u32, tok_type: u32) -> SemanticToken {
    let delta_line = line - prev_line;
    let delta_start = if delta_line == 0 { start - prev_start } else { start };
    SemanticToken {
        delta_line,
        delta_start,
        length: len,
        token_type: tok_type,
        token_modifiers_bitset: 0,
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = LspService::new(|client| HttpLsp { client });
    Server::new(stdin, stdout, socket).serve(service).await;
}
