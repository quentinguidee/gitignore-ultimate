use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "gitignore.pest"]
pub struct GitignoreParser;

#[cfg(test)]
mod tests {
    use super::*;
    use pest::{consumes_to, parses_to};

    #[test]
    fn test_line() {
        parses_to! {
            parser: GitignoreParser,
            input: "a/b/c\n",
            rule: Rule::line,
            tokens: [
                line(0, 6, [
                    path(0, 5, [
                        segment(0, 1),
                        separator(1, 2),
                        segment(2, 3),
                        separator(3, 4),
                        segment(4, 5)
                    ]),
                ])
            ]
        }
    }

    #[test]
    fn test_line_empty() {
        parses_to! {
            parser: GitignoreParser,
            input: "\n",
            rule: Rule::line,
            tokens: [
                line(0, 1, [])
            ]
        }
    }

    #[test]
    fn test_comment() {
        parses_to! {
            parser: GitignoreParser,
            input: "# a comment\n",
            rule: Rule::comment,
            tokens: [
                comment(0, 11)
            ]
        }
    }

    #[test]
    fn test_comment_escaped() {
        parses_to! {
            parser: GitignoreParser,
            input: "\\#not/a/comment\n",
            rule: Rule::line,
            tokens: [
                line(0, 16, [
                    path(0, 15, [
                        segment(0, 5),
                        separator(5, 6),
                        segment(6, 7),
                        separator(7, 8),
                        segment(8, 15)
                    ]),
                ])
            ]
        }
    }

    #[test]
    #[should_panic]
    fn test_line_path_error() {
        GitignoreParser::parse(Rule::line, "a//b/c").unwrap();
    }

    #[test]
    fn test_path() {
        parses_to! {
            parser: GitignoreParser,
            input: "a/b/c",
            rule: Rule::path,
            tokens: [
                path(0, 5, [
                    segment(0, 1),
                    separator(1, 2),
                    segment(2, 3),
                    separator(3, 4),
                    segment(4, 5)
                ])
            ]
        }
    }

    #[test]
    fn test_path_inverted() {
        parses_to! {
            parser: GitignoreParser,
            input: "!a/b/c",
            rule: Rule::path,
            tokens: [
                path(0, 6, [
                    segment(1, 2),
                    separator(2, 3),
                    segment(3, 4),
                    separator(4, 5),
                    segment(5, 6)
                ])
            ]
        }
    }

    #[test]
    fn test_path_inverted_escaped() {
        let code = GitignoreParser::parse(Rule::path, "\\!a/b/c").unwrap();
        assert_eq!(code.as_str(), "\\!a/b/c");

        parses_to! {
            parser: GitignoreParser,
            input: "\\!a/b/c",
            rule: Rule::path,
            tokens: [
                path(0, 7, [
                    segment(0, 3),
                    separator(3, 4),
                    segment(4, 5),
                    separator(5, 6),
                    segment(6, 7)
                ])
            ]
        }
    }
}
