use crate::ast::AST;
use crate::parser::parser;
use chumsky::prelude::*;
use dashmap::DashMap;
use tokio::io::{stdin, stdout};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::{
    CompletionOptions, CompletionParams, CompletionResponse, Diagnostic, DiagnosticSeverity,
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    InitializeParams, InitializeResult, InitializedParams, MessageType, OneOf, Position, Range,
    ServerCapabilities, TextDocumentIdentifier, TextDocumentItem, TextDocumentSyncCapability,
    TextDocumentSyncKind, VersionedTextDocumentIdentifier, WorkspaceFoldersServerCapabilities,
    WorkspaceServerCapabilities,
};
use tower_lsp::{Client, LanguageServer, LspService, Server};
use url::Url;

use crate::workspace::Workspace;

mod ast;
mod file;
mod parser;
mod workspace;

#[derive(Debug)]
struct Backend {
    client: Client,
    workspace: Workspace,
    asts: DashMap<Url, AST>,
}

impl Backend {
    fn new(client: Client) -> Backend {
        Self {
            client,
            workspace: Workspace::new(),
            asts: DashMap::new(),
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        let capabilities = ServerCapabilities {
            workspace: Some(WorkspaceServerCapabilities {
                workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                    supported: Some(true),
                    change_notifications: Some(OneOf::Left(true)),
                }),
                ..Default::default()
            }),
            completion_provider: Some(CompletionOptions {
                trigger_characters: Some(vec!["/".into()]),
                ..Default::default()
            }),
            text_document_sync: Some(TextDocumentSyncCapability::Kind(
                TextDocumentSyncKind::INCREMENTAL,
            )),
            ..Default::default()
        };

        Ok(InitializeResult {
            capabilities,
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "server initialized successfully")
            .await
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let TextDocumentItem { uri, text, .. } = params.text_document;
        self.workspace.open(uri.clone(), text);
        self.refresh_ast(uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let VersionedTextDocumentIdentifier { uri, .. } = params.text_document;
        let content_changes = params.content_changes;

        match self.workspace.apply_changes(&uri, content_changes).await {
            Ok(_) => {}
            Err(error) => return self.client.log_message(MessageType::ERROR, error).await,
        };

        self.refresh_ast(uri).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let TextDocumentIdentifier { uri, .. } = params.text_document;
        self.workspace.close(&uri);
        self.asts.remove(&uri);
    }

    async fn completion(&self, _params: CompletionParams) -> Result<Option<CompletionResponse>> {
        Ok(Some(CompletionResponse::Array(vec![])))
    }
}

impl Backend {
    async fn refresh_ast(&self, uri: Url) {
        let file = match self.workspace.files.get(&uri.to_string()) {
            Some(file) => file,
            None => {
                let error = format!(
                    "The file {url} is not found in the workspace.",
                    url = uri.to_string()
                );
                return self.client.log_message(MessageType::ERROR, error).await;
            }
        };

        let text = file.get_content();

        let parser = parser();
        let (out, err) = parser.parse(text.as_str()).into_output_errors();

        let no_errors = err.is_empty();
        let errors = err.into_iter();

        for error in errors {
            let span = error.span();
            let position = Position::new(span.start as u32, span.end as u32);
            let diagnostic = Diagnostic::new(
                Range::new(position, position),
                Some(DiagnosticSeverity::ERROR),
                None,
                Some("Gitignore Ultimate".to_string()),
                error.to_string(),
                None,
                None,
            );
            self.client
                .publish_diagnostics(uri.clone(), vec![diagnostic], None)
                .await;
        }

        if no_errors {
            self.client
                .publish_diagnostics(uri.clone(), vec![], None)
                .await;
        }

        if out.is_some() {
            let ast = AST::parse(out.unwrap());
            self.asts.insert(uri, ast);
        }
    }
}

#[tokio::main]
async fn main() {
    let stdin = stdin();
    let stdout = stdout();

    let (service, socket) = LspService::new(Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}
