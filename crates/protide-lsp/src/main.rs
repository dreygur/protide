mod backend;
mod code_actions;
mod completion;
mod diagnostics;
mod formatting;
mod hover;
mod inlay_hints;
mod semantic_tokens;
mod symbols;
mod workspace;

use backend::HttpLsp;
use tower_lsp::{LspService, Server};

#[tokio::main]
async fn main() {
    env_logger::init();
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = LspService::new(|client| HttpLsp::new(client));
    Server::new(stdin, stdout, socket).serve(service).await;
}
