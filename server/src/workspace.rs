use std::fmt::{Debug, Formatter};

use dashmap::DashMap;
use tower_lsp::lsp_types::TextDocumentContentChangeEvent;
use tower_lsp::lsp_types::Url;

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
    pub fn new() -> Self {
        Workspace {
            files: DashMap::new(),
        }
    }

    pub fn open(&self, uri: Url, text: String) {
        let file = File::new(uri, text);
        self.files.insert(file.url.to_string(), file);
    }

    pub fn close(&self, uri: &Url) {
        self.files.remove(&uri.to_string());
    }

    pub async fn apply_changes(
        &self,
        uri: &Url,
        content_changes: Vec<TextDocumentContentChangeEvent>,
    ) -> Result<(), String> {
        let mut file = match self.files.get_mut(&uri.to_string()) {
            Some(file) => file,
            None => Err(format!(
                "The file {url} is not opened on the server.",
                url = uri.to_string()
            ))?,
        };

        for change in content_changes {
            file.apply_change(change)
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use tower_lsp::lsp_types::Url;

    use super::Workspace;

    #[test]
    fn it_can_add_and_remove_files() {
        let workspace = Workspace::new();

        assert_eq!(workspace.files.len(), 0);

        let urls = vec![
            Url::parse("file:///a").unwrap(),
            Url::parse("file:///b").unwrap(),
        ];

        workspace.open(urls[0].clone(), "content".to_string());
        workspace.open(urls[1].clone(), "content".to_string());

        assert_eq!(workspace.files.len(), 2);

        workspace.close(&urls[1]);

        assert_eq!(workspace.files.len(), 1);
    }
}
