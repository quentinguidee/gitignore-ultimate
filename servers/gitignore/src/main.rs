use dashmap::DashMap;
use tokio::io::{stdin, stdout};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::{
    CompletionOptions, CompletionParams, CompletionResponse, DidChangeTextDocumentParams,
    DidCloseTextDocumentParams, DidOpenTextDocumentParams, InitializeParams, InitializeResult,
    InitializedParams, MessageType, OneOf, ServerCapabilities, TextDocumentIdentifier,
    TextDocumentItem, TextDocumentSyncCapability, TextDocumentSyncKind,
    VersionedTextDocumentIdentifier, WorkspaceFoldersServerCapabilities,
    WorkspaceServerCapabilities,
};
use tower_lsp::{Client, LanguageServer, LspService, Server};
use url::Url;

use lsp_workspace::workspace::Workspace;

use crate::ast::AST;
use crate::features::completions::Completions;
use crate::features::diagnostics::Diagnostics;
use crate::parser::ParseTree;

mod ast;
mod features;
mod parser;

#[derive(Debug)]
struct Backend {
    client: Client,
    workspace: Workspace,
    parse_trees: DashMap<Url, ParseTree>,
    asts: DashMap<Url, AST>,
}

impl Backend {
    fn new(client: Client) -> Backend {
        Self {
            client,
            workspace: Workspace::new(),
            parse_trees: DashMap::new(),
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
        self.parse_trees.remove(&uri);
        self.asts.remove(&uri);
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let url = params.text_document_position.text_document.uri;
        let parse_tree = match self.parse_trees.get(&url) {
            Some(ast) => ast,
            None => return Ok(None),
        };
        let file = match self.workspace.files.get(&url.to_string()) {
            Some(file) => file,
            None => return Ok(None),
        };

        let pos = params.text_document_position.position;
        let index = file.get_offset_at(pos.line, pos.character);
        Ok(Completions::generate(&parse_tree, index))
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

        let (out, err) = ParseTree::generate_from(text.as_str());

        let diagnostics = Diagnostics::generate(err, &file);
        self.client
            .publish_diagnostics(uri.clone(), diagnostics, None)
            .await;

        if out.is_some() {
            let out = out.unwrap();
            self.parse_trees.insert(uri.clone(), out.clone());
            let ast = AST::generate_from(out);
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
