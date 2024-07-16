use std::fmt::{self, Display};

use regex::Regex;
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, PartialEq)]
pub(crate) struct Footer {
    pub(crate) key: FooterKey,
    pub(crate) value: String,
}

#[derive(Debug, PartialEq)]
pub(crate) enum FooterKey {
    BreakingChange,
    String(String),
}

impl From<&str> for FooterKey {
    fn from(value: &str) -> Self {
        match value {
            "BREAKING CHANGE" | "BREAKING-CHANGE" => Self::BreakingChange,
            _ => Self::String(value.to_owned()),
        }
    }
}

impl Display for FooterKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FooterKey::BreakingChange => write!(f, "BREAKING CHANGE"),
            FooterKey::String(s) => write!(f, "{}", s),
        }
    }
}

#[derive(Debug, PartialEq, Serialize)]
pub(crate) struct Reference {
    pub(crate) action: Option<String>,
    pub(crate) prefix: String,
    pub(crate) issue: String,
}

#[derive(Debug, PartialEq)]
pub struct Commit {
    pub(crate) r#type: String,
    pub(crate) scope: Option<String>,
    pub(crate) breaking: bool,
    pub(crate) description: String,
    pub(crate) body: Option<String>,
    pub(crate) footers: Vec<Footer>,
    pub(crate) references: Vec<Reference>,
}

impl Commit {
    pub fn is_breaking(&self) -> bool {
        self.breaking
            || self
                .footers
                .iter()
                .any(|f| matches!(f.key, FooterKey::BreakingChange))
    }
}

impl fmt::Display for Commit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.r#type)
    }
}

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("missing type")]
    NoType,
    #[error("missing description")]
    NoDescription,
    #[error("empty commit message")]
    EmptyCommitMessage,
    #[error("first line doesn't match `<type>[optional scope]: <description>`")]
    InvalidFirstLine,
    #[error("scope does not match regex: {0}")]
    InvalidScope(String),
}

pub struct CommitParser {
    regex_first_line: Regex,
    regex_scope: Regex,
    regex_footer: Regex,
    regex_references: Regex,
    regex_strip: Regex,
}

impl CommitParser {
    pub fn builder() -> CommitParserBuilder {
        CommitParserBuilder::new()
    }

    pub fn parse(&self, msg: &str) -> Result<Commit, ParseError> {
        let s = self.regex_strip.replace(msg, "");
        let mut lines = s.lines();
        if let Some(first) = lines.next() {
            if let Some(capts) = self.regex_first_line.captures(first) {
                let r#type = capts.name("type").map(|t| t.as_str().to_owned());
                let scope = capts.name("scope").map(|s| s.as_str().to_owned());
                if let Some(ref scope) = scope {
                    if !self.regex_scope.is_match(scope.as_str()) {
                        return Err(ParseError::InvalidScope(
                            self.regex_scope.as_str().to_owned(),
                        ));
                    }
                }
                let breaking = capts.name("breaking").is_some();
                let description = capts.name("desc").map(|d| d.as_str().to_owned());
                match (r#type, description) {
                    (Some(r#type), Some(description)) => {
                        let mut body = String::new();
                        let mut footers: Vec<Footer> = Vec::new();
                        let mut references = Vec::new();
                        for captures in self.regex_references.captures_iter(&description) {
                            let prefix = &captures[1];
                            let issue = &captures[2];
                            let reference = Reference {
                                action: None,
                                prefix: prefix.into(),
                                issue: issue.into(),
                            };
                            references.push(reference);
                        }
                        for line in lines {
                            if let Some(capts) = self.regex_footer.captures(line) {
                                let key = capts.name("key").map(|key| key.as_str());
                                let ref_key = capts.name("ref").map(|key| key.as_str());
                                let value = capts.name("value").map(|value| value.as_str());
                                match (key, ref_key, value) {
                                    (Some(key), None, Some(value)) => {
                                        footers.push(Footer {
                                            key: key.into(),
                                            value: value.to_owned(),
                                        });
                                    }
                                    (None, Some(key), Some(value)) => {
                                        footers.push(Footer {
                                            key: key.into(),
                                            value: value.to_owned(),
                                        });
                                    }
                                    _ => unreachable!(),
                                }
                            } else if footers.is_empty() {
                                body.push_str(line);
                                body.push('\n');
                            } else if let Some(footer) = footers.last_mut() {
                                footer.value.push('\n');
                                footer.value.push_str(line);
                            }
                            for captures in self.regex_references.captures_iter(line) {
                                let prefix = &captures[1];
                                let issue = &captures[2];
                                let action = footers.last().map(|footer| footer.key.to_string());
                                let reference = Reference {
                                    action,
                                    prefix: prefix.into(),
                                    issue: issue.into(),
                                };
                                references.push(reference);
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
                            references,
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
    references_regex: String,
    strip_regex: String,
}

impl CommitParserBuilder {
    pub fn new() -> Self {
        Self {
            scope_regex: "^[[:alnum:]]+(?:[-_/][[:alnum:]]+)*$".into(),
            references_regex: "(#)([0-9]+)".into(),
            strip_regex: "".into(),
        }
    }

    pub fn scope_regex(self, scope_regex: String) -> Self {
        Self {
            scope_regex,
            references_regex: self.references_regex,
            strip_regex: self.strip_regex,
        }
    }

    pub fn references_regex(self, references_regex: String) -> Self {
        Self {
            references_regex,
            scope_regex: self.scope_regex,
            strip_regex: self.strip_regex,
        }
    }

    pub fn strip_regex(self, strip_regex: String) -> Self {
        Self {
            strip_regex,
            references_regex: self.references_regex,
            scope_regex: self.scope_regex,
        }
    }

    pub fn build(&self) -> CommitParser {
        let regex_first_line = Regex::new(
            r"(?xms)
        ^
        (?P<type>[a-zA-Z]+)
        (?:\((?P<scope>[^()\r\n]+)\))?
        (?P<breaking>!)?
        :\x20(?P<desc>[^\r\n]+)
        $",
        )
        .expect("valid scope regex");
        let regex_footer = Regex::new(
            r"(?xm)
                    ^
                    (?:(?P<key>(?:BREAKING\x20CHANGE|[a-zA-Z]+(?:-[a-zA-Z]+)*)):\x20|
                    (?P<ref>[a-zA-Z]+(?:-[a-zA-Z]+)*)\x20\#)
                    (?P<value>.+)
                    $",
        )
        .unwrap();
        let regex_scope =
            Regex::new(self.scope_regex.as_str()).expect("scope regex should be valid");
        let regex_references =
            Regex::new(self.references_regex.as_str()).expect("references regex should be valid");
        let regex_strip: Regex =
            Regex::new(self.strip_regex.as_str()).expect("strip regex should be valid");
        CommitParser {
            regex_scope,
            regex_first_line,
            regex_footer,
            regex_references,
            regex_strip,
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
                r#type: "docs".into(),
                scope: None,
                breaking: false,
                description: "correct spelling of CHANGELOG".into(),
                body: None,
                footers: Vec::new(),
                references: Vec::new(),
            }
        );
        assert!(!commit.is_breaking());
    }

    #[test]
    fn test_with_scope() {
        let msg = "feat(lang): add polish language";
        let commit: Commit = parser().parse(msg).expect("valid");
        assert_eq!(
            commit,
            Commit {
                r#type: "feat".into(),
                scope: Some("lang".into()),
                breaking: false,
                description: "add polish language".into(),
                body: None,
                footers: Vec::new(),
                references: Vec::new(),
            }
        );
        assert!(!commit.is_breaking());
    }

    #[test]
    fn test_with_complex_scope() {
        let msg = "feat(bar2/a_b-C4): add a foo to new bar";
        let commit: Commit = parser().parse(msg).expect("valid");
        assert_eq!(
            commit,
            Commit {
                r#type: "feat".into(),
                scope: Some("bar2/a_b-C4".into()),
                breaking: false,
                description: "add a foo to new bar".into(),
                body: None,
                footers: Vec::new(),
                references: Vec::new(),
            }
        );
        assert!(!commit.is_breaking());
    }

    #[test]
    fn test_with_invalid_scope() {
        let msg = "feat(invalid scope): add a foo to new bar";
        let err = parser().parse(msg).expect_err("space not allowed");
        assert_eq!(
            err.to_string(),
            "scope does not match regex: ^[[:alnum:]]+(?:[-_/][[:alnum:]]+)*$"
        );
    }

    #[test]
    fn test_with_breaking() {
        let msg = "refactor!: drop support for Node 6";
        let commit: Commit = parser().parse(msg).expect("valid");
        assert_eq!(
            commit,
            Commit {
                r#type: "refactor".into(),
                scope: None,
                breaking: true,
                description: "drop support for Node 6".into(),
                body: None,
                footers: Vec::new(),
                references: Vec::new(),
            }
        );
        assert!(commit.is_breaking());
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
                r#type: "feat".into(),
                scope: None,
                breaking: false,
                description: "allow provided config object to extend other configs".into(),
                body: None,
                footers: vec![Footer {
                    key: FooterKey::BreakingChange,
                    value:
                        "`extends` key in config file is now used for extending other config files"
                            .to_string()
                }],
                references: Vec::new(),
            }
        );
        assert!(commit.is_breaking());
    }

    #[test]
    fn test_with_breaking_footer_alias() {
        let msg = "feat: allow provided config object to extend other configs\n\
                         \n\
                         BREAKING-CHANGE: `extends` key in config file is now used for extending other config files";
        let commit: Commit = parser().parse(msg).expect("valid");
        assert!(commit.is_breaking());
    }

    #[test]
    fn test_with_breaking_footer_newline() {
        let msg = "feat: allow provided config object to extend other configs\n\
                         \n\
                         BREAKING-CHANGE: `extends` key in config\nfile is now used for extending other config files";
        let commit: Commit = parser().parse(msg).expect("valid");
        assert!(commit.is_breaking());
        assert_eq!(
            commit.footers.first().unwrap().value,
            "`extends` key in config\nfile is now used for extending other config files"
        )
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
                r#type: "fix".into(),
                scope: None,
                breaking: false,
                description: "correct minor typos in code".into(),
                body: Some("see the issue for details\n\non typos fixed.".into()),
                footers: vec![
                    Footer {
                        key: FooterKey::String("Reviewed-by".into()),
                        value: "Z".to_string()
                    },
                    Footer {
                        key: "Refs".into(),
                        value: "133".to_string()
                    }
                ],
                references: vec![Reference {
                    action: Some("Refs".into()),
                    prefix: "#".into(),
                    issue: "133".into()
                }],
            }
        );
        assert!(!commit.is_breaking());
    }

    #[test]
    fn multiple_refs() {
        let msg = "revert: let us never again speak of the noodle incident #1\n\
        \n\
        Closes: #2, #42";
        let commit: Commit = parser().parse(msg).expect("valid");
        assert_eq!(
            commit,
            Commit {
                r#type: "revert".into(),
                scope: None,
                breaking: false,
                description: "let us never again speak of the noodle incident #1".into(),
                body: None,
                footers: vec![Footer {
                    key: "Closes".into(),
                    value: "#2, #42".into()
                }],
                references: vec![
                    Reference {
                        action: None,
                        prefix: "#".into(),
                        issue: "1".into()
                    },
                    Reference {
                        action: Some("Closes".into()),
                        prefix: "#".into(),
                        issue: "2".into()
                    },
                    Reference {
                        action: Some("Closes".into()),
                        prefix: "#".into(),
                        issue: "42".into()
                    },
                ],
            }
        );
        assert!(!commit.is_breaking());
    }

    #[test]
    fn test_with_strip_prefix() {
        let msg = "Merge PR 14: docs: correct spelling of CHANGELOG";
        let commit: Commit = CommitParser::builder()
            .strip_regex("^(?:Merge PR [0-9]+: )?".to_string())
            .build()
            .parse(msg)
            .expect("valid");
        assert_eq!(
            commit,
            Commit {
                r#type: "docs".into(),
                scope: None,
                breaking: false,
                description: "correct spelling of CHANGELOG".into(),
                body: None,
                footers: Vec::new(),
                references: Vec::new(),
            }
        );
        assert!(!commit.is_breaking());
    }
}
