use crate::parser::Rule;
use pest::iterators::{Pair, Pairs};
use std::fmt::Debug;

#[derive(Debug)]
pub enum Node {
    File { children: Vec<Box<Node>> },
    Path { children: Vec<Box<Node>> },
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

#[derive(Debug)]
pub struct AST {
    pub root: Box<Node>,
}

impl AST {
    pub fn parse(pairs: Pairs<Rule>) -> AST {
        AST {
            root: AST::visit_pairs(pairs).pop().unwrap(),
        }
    }

    pub fn visit_pairs(pairs: Pairs<Rule>) -> Vec<Box<Node>> {
        return pairs.filter_map(AST::visit_pair).collect();
    }

    pub fn visit_pair(pair: Pair<Rule>) -> Option<Box<Node>> {
        match pair.as_rule() {
            Rule::file => Some(Box::new(Node::File {
                children: Self::visit_children(pair),
            })),
            Rule::line => Some(AST::visit_pair(pair.into_inner().next()?)?),
            Rule::comment => None,
            Rule::path => Some(Box::new(Node::Path {
                children: Self::visit_children(pair),
            })),
            Rule::segment => Some(Box::new(Node::Segment(pair.as_str().to_string()))),
            Rule::separator => None,
            Rule::negate => Some(Box::new(Node::Negate)),
            Rule::EOI => None,
        }
    }

    fn visit_children(pair: Pair<Rule>) -> Vec<Box<Node>> {
        return pair.into_inner().filter_map(AST::visit_pair).collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::GitignoreParser;
    use pest::Parser;

    macro_rules! assert_eq_ast {
        ($a:expr, $b:expr) => {
            let pairs = GitignoreParser::parse(Rule::file, $a).unwrap();
            let ast = AST::parse(pairs);
            assert_eq!(ast.root, $b);
        };
    }

    #[test]
    fn it_can_parse_an_empty_file() {
        assert_eq_ast!("", Box::new(Node::File { children: vec![] }));
    }

    #[test]
    fn it_can_parse_a_file_with_spaces() {
        assert_eq_ast!("   \n", Box::new(Node::File { children: vec![] }));
    }

    #[test]
    fn it_can_parse_a_file_with_path_and_spaces() {
        assert_eq_ast!(
            "a/b/c  \n",
            Box::new(Node::File {
                children: vec![Box::new(Node::Path {
                    children: vec![
                        Box::new(Node::Segment("a".to_string())),
                        Box::new(Node::Segment("b".to_string())),
                        Box::new(Node::Segment("c".to_string())),
                    ]
                })]
            })
        );
    }

    #[test]
    fn it_can_parse_a_file_with_path_and_escaped_spaces() {
        assert_eq_ast!(
            "a/b/c\\ d  \n",
            Box::new(Node::File {
                children: vec![Box::new(Node::Path {
                    children: vec![
                        Box::new(Node::Segment("a".to_string())),
                        Box::new(Node::Segment("b".to_string())),
                        Box::new(Node::Segment("c\\ d".to_string())),
                    ]
                })]
            })
        );
    }

    #[test]
    fn it_can_parse_a_file() {
        assert_eq_ast!(
            "a/b/c\n",
            Box::new(Node::File {
                children: vec![Box::new(Node::Path {
                    children: vec![
                        Box::new(Node::Segment("a".to_string())),
                        Box::new(Node::Segment("b".to_string())),
                        Box::new(Node::Segment("c".to_string())),
                    ]
                })]
            })
        );
    }

    #[test]
    fn it_can_parse_a_file_with_a_comment() {
        assert_eq_ast!(
            "a/b\n# a comment\n",
            Box::new(Node::File {
                children: vec![Box::new(Node::Path {
                    children: vec![
                        Box::new(Node::Segment("a".to_string())),
                        Box::new(Node::Segment("b".to_string())),
                    ]
                })]
            })
        );
    }

    #[test]
    fn it_can_parse_a_file_with_a_negate() {
        assert_eq_ast!(
            "!a/b\n",
            Box::new(Node::File {
                children: vec![Box::new(Node::Path {
                    children: vec![
                        Box::new(Node::Negate),
                        Box::new(Node::Segment("a".to_string())),
                        Box::new(Node::Segment("b".to_string())),
                    ]
                })]
            })
        );
    }
}
