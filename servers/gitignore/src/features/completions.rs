use chumsky::span::SimpleSpan;
use tower_lsp::lsp_types::{CompletionItem, CompletionItemKind, CompletionResponse};

use crate::parser::{ParseTree, Token};

pub struct Completions {}

impl Completions {
    pub fn generate(parse_tree: &ParseTree, position: usize) -> Option<CompletionResponse> {
        Completions::generate_path_completions(parse_tree, position)
    }

    pub fn generate_path_completions(
        parse_tree: &ParseTree,
        position: usize,
    ) -> Option<CompletionResponse> {
        let line = parse_tree.root.get_first_child_filtered(&|t| {
            let SimpleSpan { start, end, .. } = t.get_span();
            *start <= position && *end >= position && matches!(t, Token::Path { .. })
        });
        let line = match line {
            Some(line) => line,
            None => return None,
        };
        let children = match line.get_children() {
            Some(children) => children,
            None => return None,
        };
        let current_node = match line.get_child_at_position(position) {
            Some(node) => node,
            None => return None,
        };

        let mut path = "".to_string();

        for child in children {
            if child == current_node {
                break;
            }
            match child {
                Token::Segment { value, .. } => {
                    path.push_str(value);
                }
                Token::Separator { .. } => {
                    path.push('/');
                }
                _ => {}
            }
        }

        let completions = vec![CompletionItem {
            label: path.clone(),
            kind: Some(CompletionItemKind::FILE),
            detail: Some(path.clone()),
            insert_text: Some(path),
            ..Default::default()
        }];

        Some(CompletionResponse::Array(completions))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_can_generate_path_completions() {
        let (parse_tree, _) = ParseTree::generate_from("a/b/c");
        let completions = Completions::generate_path_completions(&parse_tree.unwrap(), 3);
        let completions = match completions {
            Some(CompletionResponse::Array(completions)) => completions,
            _ => panic!("Expected array of completions"),
        };
        assert_eq!(completions.len(), 1);
        assert_eq!(completions[0].label, "a/b");
    }
}
