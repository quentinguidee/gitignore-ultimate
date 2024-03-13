use dashmap::DashMap;
use pest::error::LineColLocation;
use pest::Parser;
use tokio::io::{stdin, stdout};
use tokio::task::spawn;
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

use crate::ast::AST;
use crate::parser::GitignoreParser;
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
        let client = self.client.clone();
        let uri_clone = uri.clone();

        // Spawn a new task to use the parser, since its result is not `Send`.
        let handle = spawn(async move {
            match GitignoreParser::parse(parser::Rule::file, text.as_str()) {
                Ok(ast_pest) => {
                    let ast = AST::parse(ast_pest);
                    (Some(ast), None)
                }
                Err(error) => {
                    let range = match error.line_col {
                        LineColLocation::Pos((line, col)) => {
                            let position = Position::new(line as u32 - 1, col as u32 - 1);
                            Range::new(position, position)
                        }
                        LineColLocation::Span((start_line, start_col), (end_line, end_col)) => {
                            let start = Position::new(start_line as u32 - 1, start_col as u32 - 1);
                            let end = Position::new(end_line as u32 - 1, end_col as u32 - 1);
                            Range::new(start, end)
                        }
                    };

                    (
                        None,
                        Some(vec![Diagnostic::new(
                            range,
                            Some(DiagnosticSeverity::ERROR),
                            None,
                            Some("Gitignore Ultimate".to_string()),
                            error.variant.message().to_string(),
                            None,
                            None,
                        )]),
                    )
                }
            }
        });

        let (ast, diagnostics) = handle.await.unwrap();

        if let Some(diagnostics) = diagnostics {
            client
                .publish_diagnostics(uri.clone(), diagnostics, None)
                .await;
            return;
        } else if let Some(ast) = ast {
            self.asts.insert(uri, ast);
            client.publish_diagnostics(uri_clone, vec![], None).await;
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
