use chumsky::{extra, IterParser, Parser};
use chumsky::prelude::{any, end, just, Rich, skip_then_retry_until};
use chumsky::primitive::choice;
use chumsky::span::SimpleSpan;
use chumsky::text::newline;

use crate::parser::Token::{Comment, Path};

#[derive(Debug, Clone)]
pub struct ParseTree {
    pub root: Token,
}

impl ParseTree {
    pub fn generate_from(input: &str) -> (Option<ParseTree>, Vec<Rich<char>>) {
        let (root, errors) = parser().parse(input).into_output_errors();
        let tree = match root {
            Some(root) => Some(ParseTree { root }),
            None => None,
        };
        (tree, errors)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    File {
        span: SimpleSpan,
        children: Vec<Token>,
    },
    Path {
        span: SimpleSpan,
        children: Vec<Token>,
    },
    Segment {
        span: SimpleSpan,
        value: String,
    },
    Separator {
        span: SimpleSpan,
    },
    Negate {
        span: SimpleSpan,
    },
    Comment {
        span: SimpleSpan,
    },
}

impl Token {
    pub fn get_children(&self) -> Option<&Vec<Token>> {
        match self {
            Token::File { children, .. } => Some(children),
            Token::Path { children, .. } => Some(children),
            _ => None,
        }
    }

    pub fn get_span(&self) -> &SimpleSpan {
        match self {
            Token::File { span, .. } => span,
            Token::Path { span, .. } => span,
            Token::Segment { span, .. } => span,
            Token::Separator { span, .. } => span,
            Token::Negate { span, .. } => span,
            Token::Comment { span, .. } => span,
        }
    }

    pub fn get_child_at_position(&self, position: usize) -> Option<&Token> {
        let children = self.get_children()?;
        for child in children {
            let SimpleSpan { start, end, .. } = child.get_span();
            if *start <= position && (*start == *end || *end > position) {
                return match child {
                    Token::File { .. } | Token::Path { .. } => {
                        child.get_child_at_position(position)
                    }
                    _ => Some(child),
                };
            }
        }
        Some(self)
    }

    pub fn get_all_children_filtered<F>(&self, filter: &F) -> Vec<&Token>
    where
        F: Fn(&Token) -> bool,
    {
        let mut results = vec![];
        let children = self.get_children();
        if let Some(children) = children {
            for child in children {
                if filter(child) {
                    results.push(child);
                }
                results.extend(child.get_all_children_filtered(filter));
            }
        }
        results
    }

    pub fn get_first_child_filtered<F>(&self, filter: &F) -> Option<&Token>
    where
        F: Fn(&Token) -> bool,
    {
        let children = self.get_children();
        if let Some(children) = children {
            for child in children {
                if filter(child) {
                    return Some(child);
                }
                if let Some(child) = child.get_first_child_filtered(filter) {
                    return Some(child);
                }
            }
        }
        None
    }
}

pub fn parser<'a>() -> impl Parser<'a, &'a str, Token, extra::Err<Rich<'a, char>>> {
    let comment = just("#")
        .then(any().and_is(newline().not()).and_is(end().not()).repeated())
        .map_with(|_, e| Comment { span: e.span() });

    let segment = any()
        .and_is(just("/").not())
        .and_is(newline().not())
        .and_is(end().not())
        .repeated()
        .at_least(1)
        .collect::<String>()
        .map(|x| x.trim().to_string())
        .map_with(|x, e| Token::Segment {
            span: e.span(),
            value: x,
        });

    let negate = just("!").map_with(|_, e| Token::Negate { span: e.span() });
    let separator = just("/").map_with(|_, e| Token::Separator { span: e.span() });

    let path = negate
        .or_not()
        .then(segment.or_not())
        .then(
            separator
                .then(segment)
                .repeated()
                .collect::<Vec<_>>()
                .map(|x| {
                    x.into_iter()
                        .map(|(a, b)| vec![a, b])
                        .flatten()
                        .collect::<Vec<_>>()
                }),
        )
        .then(separator.or_not())
        .map_with(|(((a, b), c), d), e| {
            let mut children = vec![];
            if let Some(a) = a {
                children.push(a);
            }
            if let Some(b) = b {
                children.push(b);
            }
            children.extend(c);
            if let Some(d) = d {
                children.push(d);
            }
            Path {
                span: e.span(),
                children,
            }
        });

    let lines = choice((comment, path))
        .separated_by(newline().recover_with(skip_then_retry_until(
            any().ignored(),
            newline().ignored().or(end()).ignored(),
        )))
        .collect::<Vec<_>>()
        .map_with(|children, e| Token::File {
            span: e.span(),
            children,
        });

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path() {
        let (parse_tree, err) = ParseTree::generate_from("a/b/c/d/e");

        assert_eq!(parse_tree.is_some(), true);
        assert_eq!(err.len(), 0);
        assert_eq!(
            parse_tree.unwrap().root,
            Token::File {
                span: SimpleSpan::new(0, 9),
                children: vec![Token::Path {
                    span: SimpleSpan::new(0, 9),
                    children: vec![
                        Token::Segment {
                            span: SimpleSpan::new(0, 1),
                            value: "a".to_string(),
                        },
                        Token::Separator {
                            span: SimpleSpan::new(1, 2)
                        },
                        Token::Segment {
                            span: SimpleSpan::new(2, 3),
                            value: "b".to_string(),
                        },
                        Token::Separator {
                            span: SimpleSpan::new(3, 4)
                        },
                        Token::Segment {
                            span: SimpleSpan::new(4, 5),
                            value: "c".to_string(),
                        },
                        Token::Separator {
                            span: SimpleSpan::new(5, 6)
                        },
                        Token::Segment {
                            span: SimpleSpan::new(6, 7),
                            value: "d".to_string(),
                        },
                        Token::Separator {
                            span: SimpleSpan::new(7, 8)
                        },
                        Token::Segment {
                            span: SimpleSpan::new(8, 9),
                            value: "e".to_string(),
                        },
                    ]
                }]
            }
        );
    }

    #[test]
    fn test_path_all_separators() {
        let (parse_tree, err) = ParseTree::generate_from("/a/b/c/");

        assert_eq!(parse_tree.is_some(), true);
        assert_eq!(err.len(), 0);
        assert_eq!(
            parse_tree.unwrap().root,
            Token::File {
                span: SimpleSpan::new(0, 7),
                children: vec![Token::Path {
                    span: SimpleSpan::new(0, 7),
                    children: vec![
                        Token::Separator {
                            span: SimpleSpan::new(0, 1)
                        },
                        Token::Segment {
                            span: SimpleSpan::new(1, 2),
                            value: "a".to_string(),
                        },
                        Token::Separator {
                            span: SimpleSpan::new(2, 3)
                        },
                        Token::Segment {
                            span: SimpleSpan::new(3, 4),
                            value: "b".to_string(),
                        },
                        Token::Separator {
                            span: SimpleSpan::new(4, 5)
                        },
                        Token::Segment {
                            span: SimpleSpan::new(5, 6),
                            value: "c".to_string(),
                        },
                        Token::Separator {
                            span: SimpleSpan::new(6, 7)
                        },
                    ]
                }]
            }
        );
    }

    #[test]
    fn test_line_empty() {
        let (parse_tree, err) = ParseTree::generate_from("\n\n");

        assert_eq!(parse_tree.is_some(), true);
        assert_eq!(err.len(), 0);
        assert_eq!(
            parse_tree.unwrap().root,
            Token::File {
                span: SimpleSpan::new(0, 2),
                children: vec![
                    Path {
                        span: SimpleSpan::new(0, 0),
                        children: vec![]
                    },
                    Path {
                        span: SimpleSpan::new(1, 1),
                        children: vec![]
                    },
                    Path {
                        span: SimpleSpan::new(2, 2),
                        children: vec![]
                    }
                ]
            }
        );
    }

    #[test]
    fn test_comment() {
        let (parse_tree, err) = ParseTree::generate_from("# a comment");

        assert_eq!(parse_tree.is_some(), true);
        assert_eq!(err.len(), 0);
        assert_eq!(
            parse_tree.unwrap().root,
            Token::File {
                span: SimpleSpan::new(0, 11),
                children: vec![Comment {
                    span: SimpleSpan::new(0, 11)
                }]
            }
        );
    }

    #[test]
    fn test_comment_escaped() {
        let (parse_tree, err) = ParseTree::generate_from("\\#not/a/comment");

        assert_eq!(parse_tree.is_some(), true);
        assert_eq!(err.len(), 0);
        assert_eq!(
            parse_tree.unwrap().root,
            Token::File {
                span: SimpleSpan::new(0, 15),
                children: vec![Path {
                    span: SimpleSpan::new(0, 15),
                    children: vec![
                        Token::Segment {
                            span: SimpleSpan::new(0, 5),
                            value: "\\#not".to_string(),
                        },
                        Token::Separator {
                            span: SimpleSpan::new(5, 6)
                        },
                        Token::Segment {
                            span: SimpleSpan::new(6, 7),
                            value: "a".to_string(),
                        },
                        Token::Separator {
                            span: SimpleSpan::new(7, 8)
                        },
                        Token::Segment {
                            span: SimpleSpan::new(8, 15),
                            value: "comment".to_string(),
                        },
                    ]
                }]
            }
        );
    }

    #[test]
    #[should_panic]
    fn test_path_error() {
        parser().parse("a//b/c").unwrap();
    }

    #[test]
    #[should_panic]
    fn test_path_starting_with_double_separator() {
        parser().parse("//b/c").unwrap();
    }

    #[test]
    fn test_path_inverted() {
        let (parse_tree, err) = ParseTree::generate_from("!a/b/c");

        assert_eq!(parse_tree.is_some(), true);
        assert_eq!(err.len(), 0);
        assert_eq!(
            parse_tree.unwrap().root,
            Token::File {
                span: SimpleSpan::new(0, 6),
                children: vec![Path {
                    span: SimpleSpan::new(0, 6),
                    children: vec![
                        Token::Negate {
                            span: SimpleSpan::new(0, 1)
                        },
                        Token::Segment {
                            span: SimpleSpan::new(1, 2),
                            value: "a".to_string(),
                        },
                        Token::Separator {
                            span: SimpleSpan::new(2, 3)
                        },
                        Token::Segment {
                            span: SimpleSpan::new(3, 4),
                            value: "b".to_string(),
                        },
                        Token::Separator {
                            span: SimpleSpan::new(4, 5)
                        },
                        Token::Segment {
                            span: SimpleSpan::new(5, 6),
                            value: "c".to_string(),
                        },
                    ]
                }]
            }
        );
    }

    #[test]
    fn test_path_inverted_escaped() {
        let (parse_tree, err) = ParseTree::generate_from("\\!a/b/c");

        assert_eq!(parse_tree.is_some(), true);
        assert_eq!(err.len(), 0);
        assert_eq!(
            parse_tree.unwrap().root,
            Token::File {
                span: SimpleSpan::new(0, 7),
                children: vec![Path {
                    span: SimpleSpan::new(0, 7),
                    children: vec![
                        Token::Segment {
                            span: SimpleSpan::new(0, 3),
                            value: "\\!a".to_string(),
                        },
                        Token::Separator {
                            span: SimpleSpan::new(3, 4)
                        },
                        Token::Segment {
                            span: SimpleSpan::new(4, 5),
                            value: "b".to_string(),
                        },
                        Token::Separator {
                            span: SimpleSpan::new(5, 6)
                        },
                        Token::Segment {
                            span: SimpleSpan::new(6, 7),
                            value: "c".to_string(),
                        },
                    ]
                }]
            }
        );
    }

    #[test]
    fn test_get_all_children_filtered() {
        let (parse_tree, _) = ParseTree::generate_from("!a/b/c\n/d/e");
        let parse_tree = parse_tree.unwrap();

        let segments = parse_tree
            .root
            .get_all_children_filtered(&|t| matches!(t, Token::Segment { .. }));
        assert_eq!(segments.len(), 5);

        let negates = parse_tree
            .root
            .get_all_children_filtered(&|t| matches!(t, Token::Negate { .. }));
        assert_eq!(negates.len(), 1);
    }

    #[test]
    fn test_get_child_at_position() {
        let (parse_tree, _) = ParseTree::generate_from("!a/b/c\n/d/e");
        let parse_tree = parse_tree.unwrap();

        let segment = parse_tree.root.get_child_at_position(1).unwrap();
        assert_eq!(
            segment,
            &Token::Segment {
                span: SimpleSpan::new(1, 2),
                value: "a".to_string(),
            }
        );

        let negate = parse_tree.root.get_child_at_position(0).unwrap();
        assert_eq!(
            negate,
            &Token::Negate {
                span: SimpleSpan::new(0, 1),
            }
        );

        let (parse_tree, _) = ParseTree::generate_from("\n\n");
        let parse_tree = parse_tree.unwrap();

        let first = parse_tree.root.get_child_at_position(0).unwrap();
        assert_eq!(
            first,
            &Token::Path {
                span: SimpleSpan::new(0, 0),
                children: vec![]
            }
        );
    }
}
