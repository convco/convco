use std::fmt;

use regex::Regex;

#[derive(Debug, PartialEq)]
pub(crate) enum Type {
    Build,
    Chore,
    Ci,
    Docs,
    Feat,
    Fix,
    Perf,
    Refactor,
    Revert,
    Style,
    Test,
    Custom(String),
}

impl AsRef<str> for Type {
    fn as_ref(&self) -> &str {
        match self {
            Self::Build => "build",
            Self::Chore => "chore",
            Self::Ci => "ci",
            Self::Docs => "docs",
            Self::Feat => "feat",
            Self::Fix => "fix",
            Self::Perf => "perf",
            Self::Refactor => "refactor",
            Self::Revert => "revert",
            Self::Style => "style",
            Self::Test => "test",
            Self::Custom(c) => c.as_str(),
        }
    }
}

impl From<&str> for Type {
    fn from(s: &str) -> Type {
        match s.to_ascii_lowercase().as_str() {
            "build" => Self::Build,
            "chore" => Self::Chore,
            "ci" => Self::Ci,
            "docs" => Self::Docs,
            "feat" => Self::Feat,
            "fix" => Self::Fix,
            "perf" => Self::Perf,
            "refactor" => Self::Refactor,
            "revert" => Self::Revert,
            "style" => Self::Style,
            "test" => Self::Test,
            custom => Self::Custom(custom.to_owned()),
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_ref())
    }
}

#[derive(Debug, PartialEq)]
pub(crate) struct Footer {
    pub(crate) key: String,
    pub(crate) value: String,
}

#[derive(Debug, PartialEq)]
pub struct Commit {
    pub(crate) r#type: Type,
    pub(crate) scope: Option<String>,
    pub(crate) breaking: bool,
    pub(crate) description: String,
    pub(crate) body: Option<String>,
    pub(crate) footers: Vec<Footer>,
}

impl fmt::Display for Commit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.r#type)
    }
}

#[derive(Debug)]
pub enum ParseError {
    NoType,
    NoDescription,
    EmptyCommitMessage,
    InvalidFirstLine,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::NoType => write!(f, "missing type"),
            ParseError::NoDescription => write!(f, "missing description"),
            ParseError::EmptyCommitMessage => write!(f, "empty commit message"),
            ParseError::InvalidFirstLine => write!(
                f,
                "first line doesn't match `<type>[optional scope]: <description>`"
            ),
        }
    }
}

impl std::error::Error for ParseError {}

pub struct CommitParser {
    regex_first_line: Regex,
    regex_footer: Regex,
}

impl CommitParser {
    pub fn builder() -> CommitParserBuilder {
        CommitParserBuilder::new()
    }

    pub fn parse(&self, s: &str) -> Result<Commit, ParseError> {
        let mut lines = s.lines();
        if let Some(first) = lines.next() {
            if let Some(capts) = self.regex_first_line.captures(first) {
                let r#type: Option<Type> = capts.name("type").map(|t| t.as_str().into());
                let scope = capts.name("scope").map(|s| s.as_str().to_owned());
                let breaking = capts.name("breaking").is_some();
                let description = capts.name("desc").map(|d| d.as_str().to_owned());
                match (r#type, description) {
                    (Some(r#type), Some(description)) => {
                        let mut body = String::new();
                        let mut footers: Vec<Footer> = Vec::new();
                        for line in lines {
                            if let Some(capts) = self.regex_footer.captures(line) {
                                let key = capts.name("key").map(|key| key.as_str());
                                let ref_key = capts.name("ref").map(|key| key.as_str());
                                let value = capts.name("value").map(|value| value.as_str());
                                match (key, ref_key, value) {
                                    (Some(key), None, Some(value)) => {
                                        footers.push(Footer {
                                            key: key.to_owned(),
                                            value: value.to_owned(),
                                        });
                                    }
                                    (None, Some(key), Some(value)) => {
                                        footers.push(Footer {
                                            key: key.to_owned(),
                                            value: value.to_owned(),
                                        });
                                    }
                                    _ => unreachable!(),
                                }
                            } else if footers.is_empty() {
                                body.push_str(line);
                                body.push('\n');
                            } else if let Some(v) = footers.last_mut() {
                                v.value.push_str(line);
                                v.value.push('\n');
                            }
                        }
                        let body = if body.trim().is_empty() {
                            None
                        } else {
                            Some(body.trim().to_owned())
                        };
                        Ok(Commit {
                            r#type,
                            scope,
                            breaking,
                            description,
                            body,
                            footers,
                        })
                    }
                    (None, _) => Err(ParseError::NoType),
                    (_, None) => Err(ParseError::NoDescription),
                }
            } else {
                Err(ParseError::InvalidFirstLine)
            }
        } else {
            Err(ParseError::EmptyCommitMessage)
        }
    }
}

pub struct CommitParserBuilder {
    scope_regex: String,
}

impl CommitParserBuilder {
    pub fn new() -> Self {
        Self {
            scope_regex: "[[:alnum:]]+(?:[-_/][[:alnum:]]+)*".into(),
        }
    }

    pub fn scope_regex(self, scope_regex: String) -> Self {
        Self { scope_regex }
    }

    pub fn build(&self) -> CommitParser {
        let regex_first_line = Regex::new(&format!(
            r#"(?xms)
        ^
        (?P<type>[a-zA-Z]+)
        (?:\((?P<scope>{})\))?
        (?P<breaking>!)?
        :\x20(?P<desc>[^\r\n]+)
        $"#,
            self.scope_regex
        ))
        .expect("valid scope regex");
        let regex_footer = Regex::new(
            r#"(?xm)
                    ^
                    (?:(?P<key>(?:BREAKING\x20CHANGE|[a-zA-Z]+(?:-[a-zA-Z]+)*)):\x20|
                    (?P<ref>[a-zA-Z]+(?:-[a-zA-Z]+)*)\x20\#)
                    (?P<value>.+)
                    $"#,
        )
        .unwrap();
        CommitParser {
            regex_first_line,
            regex_footer,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parser() -> CommitParser {
        CommitParser::builder().build()
    }

    #[test]
    fn test_simple() {
        let msg = "docs: correct spelling of CHANGELOG";
        let commit: Commit = parser().parse(msg).expect("valid");
        assert_eq!(
            commit,
            Commit {
                r#type: Type::Docs,
                scope: None,
                breaking: false,
                description: "correct spelling of CHANGELOG".into(),
                body: None,
                footers: Vec::new()
            }
        );
    }

    #[test]
    fn test_with_scope() {
        let msg = "feat(lang): add polish language";
        let commit: Commit = parser().parse(msg).expect("valid");
        assert_eq!(
            commit,
            Commit {
                r#type: Type::Feat,
                scope: Some("lang".into()),
                breaking: false,
                description: "add polish language".into(),
                body: None,
                footers: Vec::new()
            }
        );
    }

    #[test]
    fn test_with_complex_scope() {
        let msg = "feat(bar2/a_b-C4): add a foo to new bar";
        let commit: Commit = parser().parse(msg).expect("valid");
        assert_eq!(
            commit,
            Commit {
                r#type: Type::Feat,
                scope: Some("bar2/a_b-C4".into()),
                breaking: false,
                description: "add a foo to new bar".into(),
                body: None,
                footers: Vec::new()
            }
        );
    }

    #[test]
    fn test_with_breaking() {
        let msg = "refactor!: drop support for Node 6";
        let commit: Commit = parser().parse(msg).expect("valid");
        assert_eq!(
            commit,
            Commit {
                r#type: Type::Refactor,
                scope: None,
                breaking: true,
                description: "drop support for Node 6".into(),
                body: None,
                footers: Vec::new()
            }
        );
    }

    #[test]
    fn test_with_breaking_footer() {
        let msg = "feat: allow provided config object to extend other configs\n\
                         \n\
                         BREAKING CHANGE: `extends` key in config file is now used for extending other config files";
        let commit: Commit = parser().parse(msg).expect("valid");
        assert_eq!(
            commit,
            Commit {
                r#type: Type::Feat,
                scope: None,
                breaking: false,
                description: "allow provided config object to extend other configs".into(),
                body: None,
                footers: vec![Footer {
                    key: "BREAKING CHANGE".to_string(),
                    value:
                        "`extends` key in config file is now used for extending other config files"
                            .to_string()
                }]
            }
        );
    }

    #[test]
    fn test_with_multi_body_and_footer() {
        let msg = "fix: correct minor typos in code\n\
                   \n\
                   see the issue for details\n\
                   \n\
                   on typos fixed.\n\
                   \n\
                   Reviewed-by: Z\n\
                   Refs #133";
        let commit: Commit = parser().parse(msg).expect("valid");
        assert_eq!(
            commit,
            Commit {
                r#type: Type::Fix,
                scope: None,
                breaking: false,
                description: "correct minor typos in code".into(),
                body: Some("see the issue for details\n\non typos fixed.".into()),
                footers: vec![
                    Footer {
                        key: "Reviewed-by".to_string(),
                        value: "Z".to_string()
                    },
                    Footer {
                        key: "Refs".to_string(),
                        value: "133".to_string()
                    }
                ]
            }
        );
    }
}
