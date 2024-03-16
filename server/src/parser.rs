use chumsky::prelude::{any, end, just, skip_then_retry_until, Rich};
use chumsky::primitive::choice;
use chumsky::text::newline;
use chumsky::{extra, IterParser, Parser};

use crate::parser::Token::{Comment, Path};

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    File(Vec<Token>),
    Path(Vec<Token>),
    Segment(String),
    Separator,
    Negate,
    Comment,
}

pub fn parser<'a>() -> impl Parser<'a, &'a str, Token, extra::Err<Rich<'a, char>>> {
    let comment = just("#")
        .then(any().and_is(newline().not()).and_is(end().not()).repeated())
        .map(|_| Comment);

    let segment = any()
        .and_is(just("/").not())
        .and_is(newline().not())
        .and_is(end().not())
        .repeated()
        .at_least(1)
        .collect::<String>()
        .map(|x| x.trim().to_string())
        .map(|x| Token::Segment(x));

    let negate = just("!").map(|_| Token::Negate);
    let separator = just("/").map(|_| Token::Separator);

    let path = negate
        .or_not()
        .then(separator.or_not())
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
        .map(|((((a, b), c), d), e)| {
            let mut path = vec![];
            if let Some(a) = a {
                path.push(a);
            }
            if let Some(b) = b {
                path.push(b);
            }
            if let Some(c) = c {
                path.push(c);
            }
            path.extend(d);
            if let Some(e) = e {
                path.push(e);
            }
            Path(path)
        });

    let lines = choice((comment, path))
        .separated_by(newline().recover_with(skip_then_retry_until(
            any().ignored(),
            newline().ignored().or(end()).ignored(),
        )))
        .collect::<Vec<_>>()
        .map(Token::File);

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path() {
        let tree = parser().parse("a/b/c/d/e").unwrap();
        assert_eq!(
            tree,
            Token::File(vec![Token::Path(vec![
                Token::Segment("a".to_string()),
                Token::Separator,
                Token::Segment("b".to_string()),
                Token::Separator,
                Token::Segment("c".to_string()),
                Token::Separator,
                Token::Segment("d".to_string()),
                Token::Separator,
                Token::Segment("e".to_string())
            ])])
        );
    }

    #[test]
    fn test_path_all_separators() {
        let tree = parser().parse("/a/b/c/").unwrap();
        assert_eq!(
            tree,
            Token::File(vec![Token::Path(vec![
                Token::Separator,
                Token::Segment("a".to_string()),
                Token::Separator,
                Token::Segment("b".to_string()),
                Token::Separator,
                Token::Segment("c".to_string()),
                Token::Separator
            ])])
        );
    }

    #[test]
    fn test_line_empty() {
        let tree = parser().parse("\n\n").unwrap();
        assert_eq!(
            tree,
            Token::File(vec![Path(vec![]), Path(vec![]), Path(vec![])])
        );
    }

    #[test]
    fn test_comment() {
        let tree = parser().parse("# a comment").unwrap();
        assert_eq!(tree, Token::File(vec![Comment]));
    }

    #[test]
    fn test_comment_escaped() {
        let tree = parser().parse("\\#not/a/comment").unwrap();
        assert_eq!(
            tree,
            Token::File(vec![Path(vec![
                Token::Segment("\\#not".to_string()),
                Token::Separator,
                Token::Segment("a".to_string()),
                Token::Separator,
                Token::Segment("comment".to_string())
            ])])
        );
    }

    #[test]
    #[should_panic]
    fn test_line_path_error() {
        parser().parse("a//b/c").unwrap();
    }

    #[test]
    fn test_path_inverted() {
        let tree = parser().parse("!a/b/c").unwrap();
        assert_eq!(
            tree,
            Token::File(vec![Path(vec![
                Token::Negate,
                Token::Segment("a".to_string()),
                Token::Separator,
                Token::Segment("b".to_string()),
                Token::Separator,
                Token::Segment("c".to_string())
            ])])
        );
    }

    #[test]
    fn test_path_inverted_escaped() {
        let tree = parser().parse("\\!a/b/c").unwrap();
        assert_eq!(
            tree,
            Token::File(vec![Path(vec![
                Token::Segment("\\!a".to_string()),
                Token::Separator,
                Token::Segment("b".to_string()),
                Token::Separator,
                Token::Segment("c".to_string())
            ])])
        );
    }
}
