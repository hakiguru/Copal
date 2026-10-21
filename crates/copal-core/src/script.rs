//! The text command language shared by the CLI, the console and `.copalscript` files.
//!
//! One command per line: `name key=value key="value with spaces"`.
//! Empty lines and lines starting with `#` are ignored.

use std::collections::BTreeMap;
use std::str::FromStr;

use crate::command::CommandError;

/// Parsed `key=value` arguments of one command line.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Args {
    command: String,
    values: BTreeMap<String, String>,
}

impl Args {
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            values: BTreeMap::new(),
        }
    }

    pub fn with(mut self, key: &str, value: impl ToString) -> Self {
        self.values.insert(key.to_string(), value.to_string());
        self
    }

    pub fn command(&self) -> &str {
        &self.command
    }

    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.values.keys().map(String::as_str)
    }

    pub fn opt_str(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }

    pub fn str(&self, key: &str) -> Result<&str, CommandError> {
        self.opt_str(key).ok_or_else(|| CommandError::MissingArg {
            command: self.command.clone(),
            arg: key.to_string(),
        })
    }

    pub fn opt<T: FromStr>(&self, key: &str) -> Result<Option<T>, CommandError>
    where
        T::Err: std::fmt::Display,
    {
        match self.values.get(key) {
            None => Ok(None),
            Some(raw) => raw
                .parse::<T>()
                .map(Some)
                .map_err(|e| CommandError::BadValue {
                    command: self.command.clone(),
                    arg: key.to_string(),
                    value: raw.clone(),
                    reason: e.to_string(),
                }),
        }
    }

    pub fn get<T: FromStr>(&self, key: &str) -> Result<T, CommandError>
    where
        T::Err: std::fmt::Display,
    {
        self.opt(key)?.ok_or_else(|| CommandError::MissingArg {
            command: self.command.clone(),
            arg: key.to_string(),
        })
    }

    /// Fails if an argument is not in `allowed` — catches typos like `sise=10`.
    pub fn only(&self, allowed: &[&str]) -> Result<(), CommandError> {
        match self.keys().find(|k| !allowed.contains(k)) {
            Some(k) => Err(CommandError::UnknownArg {
                command: self.command.clone(),
                arg: k.to_string(),
            }),
            None => Ok(()),
        }
    }
}

/// Parses one line. Returns `Ok(None)` for blank lines and comments.
pub fn parse_line(line: &str) -> Result<Option<Args>, CommandError> {
    let tokens = tokenize(line)?;
    let mut tokens = tokens.into_iter();
    let Some(name) = tokens.next() else {
        return Ok(None);
    };
    if name.contains('=') {
        return Err(CommandError::Syntax(format!(
            "line must start with a command name, got `{name}`"
        )));
    }
    let mut args = Args::new(name);
    for token in tokens {
        let Some((key, value)) = token.split_once('=') else {
            return Err(CommandError::Syntax(format!(
                "expected key=value, got `{token}`"
            )));
        };
        if key.is_empty() {
            return Err(CommandError::Syntax(format!(
                "empty argument name in `{token}`"
            )));
        }
        if args
            .values
            .insert(key.to_string(), value.to_string())
            .is_some()
        {
            return Err(CommandError::Syntax(format!(
                "argument `{key}` is given twice"
            )));
        }
    }
    Ok(Some(args))
}

fn tokenize(line: &str) -> Result<Vec<String>, CommandError> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_token = false;
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                quoted = !quoted;
                in_token = true;
            }
            '\\' if quoted => match chars.next() {
                Some(next) => current.push(next),
                None => return Err(CommandError::Syntax("line ends after `\\`".into())),
            },
            '#' if !quoted && !in_token => break,
            c if c.is_whitespace() && !quoted => {
                if in_token {
                    tokens.push(std::mem::take(&mut current));
                    in_token = false;
                }
            }
            c => {
                current.push(c);
                in_token = true;
            }
        }
    }
    if quoted {
        return Err(CommandError::Syntax("missing closing quote".into()));
    }
    if in_token {
        tokens.push(current);
    }
    Ok(tokens)
}

/// Quotes a value if it needs quoting.
pub fn quote(value: &str) -> String {
    if !value.is_empty()
        && !value
            .chars()
            .any(|c| c.is_whitespace() || c == '"' || c == '#' || c == '\\')
    {
        value.to_string()
    } else {
        let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
        format!("\"{escaped}\"")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_line() {
        let a = parse_line("object.move id=3 x=1.5 z=-2").unwrap().unwrap();
        assert_eq!(a.command(), "object.move");
        assert_eq!(a.get::<u64>("id").unwrap(), 3);
        assert_eq!(a.get::<f32>("x").unwrap(), 1.5);
        assert_eq!(a.opt::<f32>("y").unwrap(), None);
    }

    #[test]
    fn quotes_and_comments() {
        let a = parse_line(r#"project.rename name="My \"best\" mini"  # comment"#)
            .unwrap()
            .unwrap();
        assert_eq!(a.str("name").unwrap(), r#"My "best" mini"#);
        assert!(parse_line("   # only a comment").unwrap().is_none());
        assert!(parse_line("").unwrap().is_none());
    }

    #[test]
    fn quote_round_trip() {
        for s in [
            "plain",
            "with space",
            "with \"quotes\"",
            "",
            "a#b",
            r"back\slash",
        ] {
            let line = format!("x v={}", quote(s));
            assert_eq!(parse_line(&line).unwrap().unwrap().str("v").unwrap(), s);
        }
    }

    #[test]
    fn syntax_errors() {
        assert!(parse_line("cmd novalue").is_err());
        assert!(parse_line("cmd a=1 a=2").is_err());
        assert!(parse_line(r#"cmd a="open"#).is_err());
        assert!(parse_line("a=1").is_err());
        assert!(parse_line("cmd =1").is_err());
    }

    #[test]
    fn bad_value_names_the_argument() {
        let a = parse_line("object.move id=abc").unwrap().unwrap();
        let err = a.get::<u64>("id").unwrap_err();
        assert!(matches!(err, CommandError::BadValue { ref arg, .. } if arg == "id"));
    }

    #[test]
    fn only_rejects_typos() {
        let a = parse_line("object.add_cube sise=10").unwrap().unwrap();
        assert!(matches!(
            a.only(&["size"]),
            Err(CommandError::UnknownArg { .. })
        ));
    }
}
