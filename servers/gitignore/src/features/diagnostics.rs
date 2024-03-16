use chumsky::error::Rich;
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};

use lsp_workspace::file::File;

pub struct Diagnostics {}

impl Diagnostics {
    pub fn generate(errors: Vec<Rich<char>>, file: &File) -> Vec<Diagnostic> {
        let errors = errors.into_iter();
        let mut diagnostics = vec![];
        for error in errors {
            let span = error.span();

            let (start_line, start_char) = file.get_position_at(span.start);
            let (end_line, end_char) = file.get_position_at(span.end);

            let range = Range::new(
                Position::new(start_line, start_char),
                Position::new(end_line, end_char),
            );

            diagnostics.push(Diagnostic::new(
                range,
                Some(DiagnosticSeverity::ERROR),
                None,
                Some("Gitignore Ultimate".to_string()),
                error.to_string(),
                None,
                None,
            ));
        }
        diagnostics
    }
}
