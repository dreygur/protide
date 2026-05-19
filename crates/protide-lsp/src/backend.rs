use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use crate::code_actions::code_actions;
use crate::completion::complete;
use crate::diagnostics::compute_diagnostics;
use crate::formatting::format_document;
use crate::hover::hover_at;
use crate::inlay_hints::inlay_hints;
use crate::semantic_tokens::tokenize;
use crate::symbols::{
    document_symbols, goto_definition_at, parse_annotation_value, prepare_rename_at, rename_symbol,
};
use crate::workspace::{workspace_goto_depends, workspace_symbols};

pub struct HttpLsp {
    pub client: Client,
    pub docs: Arc<RwLock<HashMap<Url, String>>>,
    root_uri: OnceLock<Url>,
}

impl HttpLsp {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            docs: Arc::new(RwLock::new(HashMap::new())),
            root_uri: OnceLock::new(),
        }
    }

    pub async fn get_content(&self, uri: &Url) -> Option<String> {
        let docs = self.docs.read().await;
        docs.get(uri)
            .cloned()
            .or_else(|| std::fs::read_to_string(uri.path()).ok())
    }

    async fn publish_diags(&self, uri: Url, content: &str) {
        let diags = compute_diagnostics(content);
        self.client.publish_diagnostics(uri, diags, None).await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for HttpLsp {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        if let Some(uri) = params.root_uri {
            let _ = self.root_uri.set(uri);
        }
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![
                        "@".to_string(),
                        "#".to_string(),
                        "{".to_string(),
                    ]),
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
                document_symbol_provider: Some(OneOf::Left(true)),
                definition_provider: Some(OneOf::Left(true)),
                workspace_symbol_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Right(RenameOptions {
                    prepare_provider: Some(true),
                    work_done_progress_options: Default::default(),
                })),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                document_formatting_provider: Some(OneOf::Left(true)),
                inlay_hint_provider: Some(OneOf::Left(true)),
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

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        self.docs.write().await.insert(uri.clone(), text.clone());
        self.publish_diags(uri, &text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.into_iter().last() {
            let uri = params.text_document.uri;
            self.docs.write().await.insert(uri.clone(), change.text.clone());
            self.publish_diags(uri, &change.text).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = &params.text_document.uri;
        self.docs.write().await.remove(uri);
        self.client.publish_diagnostics(uri.clone(), vec![], None).await;
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let pos = params.text_document_position_params.position;
        let uri = &params.text_document_position_params.text_document.uri;
        let content = match self.get_content(uri).await {
            Some(c) => c,
            None => return Ok(None),
        };
        Ok(hover_at(&content, pos))
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let pos = params.text_document_position.position;
        let uri = &params.text_document_position.text_document.uri;
        let content = match self.get_content(uri).await {
            Some(c) => c,
            None => return Ok(None),
        };
        Ok(Some(CompletionResponse::Array(complete(&content, pos))))
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = &params.text_document.uri;
        let content = match self.get_content(uri).await {
            Some(c) => c,
            None => return Ok(None),
        };
        Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data: tokenize(&content),
        })))
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = &params.text_document.uri;
        let content = match self.get_content(uri).await {
            Some(c) => c,
            None => return Ok(None),
        };
        Ok(document_symbols(&content))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let pos = params.text_document_position_params.position;
        let uri = &params.text_document_position_params.text_document.uri;
        let content = match self.get_content(uri).await {
            Some(c) => c,
            None => return Ok(None),
        };

        if let Some(result) = goto_definition_at(&content, uri, pos) {
            return Ok(Some(result));
        }

        // Multi-file: @depends not found in same file — search workspace
        if let Some(line) = content.lines().nth(pos.line as usize) {
            if let Some(dep) = parse_annotation_value(line.trim_start(), "# @depends") {
                if let Some(root) = self.root_uri.get() {
                    if let Ok(root_path) = root.to_file_path() {
                        return Ok(workspace_goto_depends(&root_path, dep));
                    }
                }
            }
        }

        Ok(None)
    }

    async fn symbol(
        &self,
        params: WorkspaceSymbolParams,
    ) -> Result<Option<Vec<SymbolInformation>>> {
        let Some(root) = self.root_uri.get() else { return Ok(None) };
        let Ok(root_path) = root.to_file_path() else { return Ok(None) };
        let symbols = workspace_symbols(&root_path, &params.query);
        Ok(if symbols.is_empty() { None } else { Some(symbols) })
    }

    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> Result<Option<PrepareRenameResponse>> {
        let uri = &params.text_document.uri;
        let content = match self.get_content(uri).await {
            Some(c) => c,
            None => return Ok(None),
        };
        Ok(prepare_rename_at(&content, params.position))
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let pos = params.text_document_position.position;
        let uri = &params.text_document_position.text_document.uri;
        let content = match self.get_content(uri).await {
            Some(c) => c,
            None => return Ok(None),
        };
        Ok(rename_symbol(&content, uri, pos, &params.new_name))
    }

    async fn code_action(
        &self,
        params: CodeActionParams,
    ) -> Result<Option<CodeActionResponse>> {
        let uri = &params.text_document.uri;
        let content = match self.get_content(uri).await {
            Some(c) => c,
            None => return Ok(None),
        };
        let actions = code_actions(&content, uri, params.range);
        Ok(if actions.is_empty() { None } else { Some(actions) })
    }

    async fn formatting(
        &self,
        params: DocumentFormattingParams,
    ) -> Result<Option<Vec<TextEdit>>> {
        let uri = &params.text_document.uri;
        let content = match self.get_content(uri).await {
            Some(c) => c,
            None => return Ok(None),
        };
        let edits = format_document(&content);
        Ok(if edits.is_empty() { None } else { Some(edits) })
    }

    async fn inlay_hint(
        &self,
        params: InlayHintParams,
    ) -> Result<Option<Vec<InlayHint>>> {
        let uri = &params.text_document.uri;
        let content = match self.get_content(uri).await {
            Some(c) => c,
            None => return Ok(None),
        };
        let hints = inlay_hints(&content, params.range);
        Ok(if hints.is_empty() { None } else { Some(hints) })
    }
}
