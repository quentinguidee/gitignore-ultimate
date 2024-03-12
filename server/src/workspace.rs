use dashmap::DashMap;
use std::fmt::{Debug, Formatter};
use tower_lsp::lsp_types::TextDocumentItem;
use tower_lsp::{
    lsp_types::{
        DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
        MessageType, Url,
    },
    Client,
};

use super::file::File;

pub struct Workspace {
    pub files: DashMap<String, File>,
}

impl Debug for Workspace {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Workspace {{ files: {} }}", self.files.len())
    }
}

impl Workspace {
    pub fn add_file(&self, file: File) {
        self.files.insert(file.url.to_string(), file);
    }

    pub fn remove_file(&self, url: Url) {
        self.files.remove(&url.to_string());
    }

    pub fn new() -> Self {
        Workspace {
            files: DashMap::new(),
        }
    }

    pub fn open(&self, params: DidOpenTextDocumentParams) {
        let TextDocumentItem { uri, text, .. } = params.text_document;
        let file = File::new(uri, text);
        self.add_file(file);
    }

    pub fn close(&self, params: DidCloseTextDocumentParams) {
        self.remove_file(params.text_document.uri);
    }

    pub async fn apply_changes(&self, params: DidChangeTextDocumentParams, client: &Client) {
        let mut file = match self.files.get_mut(&params.text_document.uri.to_string()) {
            Some(file) => file,
            None => {
                let error = format!(
                    "The file {url} is not opened on the server.",
                    url = params.text_document.uri.to_string()
                );
                client.log_message(MessageType::ERROR, error).await;
                return;
            }
        };

        for change in params.content_changes {
            file.apply_change(change)
        }
    }
}

#[cfg(test)]
mod tests {
    use tower_lsp::lsp_types::Url;

    use super::File;
    use super::Workspace;

    #[test]
    fn it_can_add_and_remove_files() {
        let workspace = Workspace::new();

        assert_eq!(workspace.files.len(), 0);

        let urls = vec![
            Url::parse("file:///a").unwrap(),
            Url::parse("file:///b").unwrap(),
        ];

        workspace.add_file(File::new(urls[0].clone(), "content".to_string()));
        workspace.add_file(File::new(urls[1].clone(), "content".to_string()));

        assert_eq!(workspace.files.len(), 2);

        workspace.remove_file(urls[1].clone());

        assert_eq!(workspace.files.len(), 1);
    }
}
