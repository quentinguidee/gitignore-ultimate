use std::path::PathBuf;

use percent_encoding::percent_decode;
use ropey::Rope;
use tower_lsp::lsp_types::{Range, TextDocumentContentChangeEvent, Url};

pub struct File {
    pub url: Url,
    pub text: Rope,
}

impl File {
    pub fn new(url: Url, text: String) -> Self {
        File {
            url,
            text: Rope::from_str(text.as_str()),
        }
    }

    pub fn get_content(&self) -> String {
        self.text.to_string()
    }

    pub fn get_line_content(&self, line_number: u32) -> String {
        self.text.line(line_number as usize).to_string()
    }

    pub fn get_position_at(&self, offset: usize) -> (u32, u32) {
        let line = self.text.char_to_line(offset);
        let character = offset - self.text.line_to_char(line);
        (line as u32, character as u32)
    }

    pub fn get_offset_at(&self, line: u32, character: u32) -> usize {
        self.text.line_to_char(line as usize) + character as usize
    }

    pub fn path(&self) -> Result<PathBuf, String> {
        let path = self.url.path();
        match percent_decode(&path.as_bytes()).decode_utf8() {
            Ok(path) => Ok(PathBuf::from(path.to_string())),
            Err(error) => Err(error.to_string()),
        }
    }

    pub fn directory(&self) -> Result<PathBuf, String> {
        let path = match self.path() {
            Ok(path) => path,
            Err(error) => return Err(error),
        };
        match path.parent() {
            Some(parent) => Ok(PathBuf::from(parent)),
            None => Err(format!("Couldn't get parent of {:?}", path)),
        }
    }

    pub fn apply_change(&mut self, change: TextDocumentContentChangeEvent) {
        let text = change.text;
        let range = match change.range {
            Some(range) => range,
            None => return,
        };

        let Range { start, end } = range;

        let start = self.text.line_to_char(start.line as usize) + (start.character as usize);
        let end = self.text.line_to_char(end.line as usize) + end.character as usize;

        self.text.remove(start..end);
        self.text.insert(start, text.as_str());
    }
}

#[cfg(test)]
mod tests {
    use tower_lsp::lsp_types::Url;

    use super::File;

    #[test]
    fn it_can_get_line_content() {
        let url = Url::parse("file:///a").unwrap();
        let content = "First line\nSecond line\nThird line".to_string();

        let file = File::new(url, content);

        let line_content = file.get_line_content(1);
        assert_eq!(line_content, "Second line\n");
    }

    #[test]
    fn it_can_get_path() {
        let url = Url::parse("file:///Users/me/file").unwrap();
        let file = File::new(url, "".to_string());

        let path = file.path().unwrap();
        let line_content = path.to_str().unwrap();
        assert_eq!(line_content, "/Users/me/file");
    }

    #[test]
    fn it_can_get_strange_windows_path() {
        let url = Url::parse("file:///c%3A/Users/me/folder\\subfolder\\filename").unwrap();
        let file = File::new(url, "content".to_string());

        assert_eq!(
            file.path().unwrap().to_str().unwrap(),
            "/c:/Users/me/folder/subfolder/filename"
        );
    }

    #[test]
    fn it_can_get_directory() {
        let url = Url::parse("file:///Users/me/file").unwrap();
        let file = File::new(url, "".to_string());

        let directory = file.directory().unwrap();
        assert_eq!(directory.to_str().unwrap(), "/Users/me");
    }
}
