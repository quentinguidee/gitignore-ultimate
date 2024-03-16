use std::fmt::Debug;

use crate::parser::{ParseTree, Token};

#[derive(Debug)]
pub enum Node {
    File { children: Vec<Node> },
    Path { children: Vec<Node> },
    Segment(String),
    Negate,
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Node::File { children: a }, Node::File { children: b }) => a == b,
            (Node::Path { children: a }, Node::Path { children: b }) => a == b,
            (Node::Segment(a), Node::Segment(b)) => a == b,
            (Node::Negate, Node::Negate) => true,
            _ => false,
        }
    }
}

impl Node {
    pub fn get_children(&self) -> Option<&Vec<Node>> {
        match &self {
            Node::File { children } => Some(children),
            Node::Path { children } => Some(children),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct AST {
    pub root: Node,
}

impl AST {
    pub fn generate_from(parse_tree: ParseTree) -> AST {
        AST {
            root: Self::visit_token(parse_tree.root).unwrap(),
        }
    }

    pub fn visit_tokens(tokens: Vec<Token>) -> Vec<Node> {
        return tokens.into_iter().filter_map(AST::visit_token).collect();
    }

    pub fn visit_token(token: Token) -> Option<Node> {
        match token {
            Token::File { children, .. } => Some(Node::File {
                children: Self::visit_tokens(children),
            }),
            Token::Path { children, .. } => {
                let children = Self::visit_tokens(children);
                if children.is_empty() {
                    None
                } else {
                    Some(Node::Path { children })
                }
            }
            Token::Segment { value, .. } => {
                if value.is_empty() {
                    None
                } else {
                    Some(Node::Segment(value))
                }
            }
            Token::Negate { .. } => Some(Node::Negate),
            Token::Separator { .. } => None,
            Token::Comment { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_eq_ast {
        ($a:expr, $b:expr) => {
            let (parse_tree, _) = ParseTree::generate_from($a);
            let ast = AST::generate_from(parse_tree.unwrap());
            assert_eq!(ast.root, $b);
        };
    }

    #[test]
    fn it_can_parse_an_empty_file() {
        assert_eq_ast!("", Node::File { children: vec![] });
    }

    #[test]
    fn it_can_parse_a_file_with_spaces() {
        assert_eq_ast!("   \n", Node::File { children: vec![] });
    }

    #[test]
    fn it_can_parse_a_file_with_path_and_spaces() {
        assert_eq_ast!(
            "a/b/c  \n",
            Node::File {
                children: vec![Node::Path {
                    children: vec![
                        Node::Segment("a".to_string()),
                        Node::Segment("b".to_string()),
                        Node::Segment("c".to_string()),
                    ]
                }]
            }
        );
    }

    #[test]
    fn it_can_parse_a_file_with_path_and_escaped_spaces() {
        assert_eq_ast!(
            "a/b/c\\ d  \n",
            Node::File {
                children: vec![Node::Path {
                    children: vec![
                        Node::Segment("a".to_string()),
                        Node::Segment("b".to_string()),
                        Node::Segment("c\\ d".to_string()),
                    ]
                }]
            }
        );
    }

    #[test]
    fn it_can_parse_path() {
        assert_eq_ast!(
            "a/b/c",
            Node::File {
                children: vec![Node::Path {
                    children: vec![
                        Node::Segment("a".to_string()),
                        Node::Segment("b".to_string()),
                        Node::Segment("c".to_string()),
                    ]
                }]
            }
        );
    }

    #[test]
    fn it_can_parse_path_with_comment() {
        assert_eq_ast!(
            "a/b\n# a comment",
            Node::File {
                children: vec![Node::Path {
                    children: vec![
                        Node::Segment("a".to_string()),
                        Node::Segment("b".to_string()),
                    ]
                }]
            }
        );
    }

    #[test]
    fn it_can_parse_path_with_negate() {
        assert_eq_ast!(
            "!a/b",
            Node::File {
                children: vec![Node::Path {
                    children: vec![
                        Node::Negate,
                        Node::Segment("a".to_string()),
                        Node::Segment("b".to_string()),
                    ]
                }]
            }
        );
    }
}
