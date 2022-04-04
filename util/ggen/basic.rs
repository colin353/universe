use crate::Result;
use crate::{take_char_while, take_while, GrammarUnit, ParseError};

#[derive(Clone, Debug, PartialEq)]
pub struct QuotedString {
    pub value: String,
    start: usize,
    end: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BareWord {
    start: usize,
    end: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Whitespace {
    start: usize,
    end: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Numeric {
    pub value: f64,
    start: usize,
    end: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Integer {
    pub value: i64,
    start: usize,
    end: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Identifier {
    start: usize,
    end: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Comment {
    start: usize,
    end: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EOF {
    pos: usize,
}

impl GrammarUnit for QuotedString {
    fn try_match(content: &str, offset: usize) -> Result<(Self, usize, Option<ParseError>)> {
        if !content.starts_with('"') {
            return Err(ParseError::expected(
                QuotedString::name(),
                offset,
                offset + 1,
            ));
        }

        let inside_start = 1;
        let inside_size = take_while(&content[inside_start..], |c| {
            if c.starts_with("\\\"") {
                return 2;
            }
            if c.starts_with('"') {
                return 0;
            }
            c.chars().next().map(|ch| ch.len_utf8()).unwrap_or(0)
        });

        let inside_end = inside_start + inside_size;
        let end = inside_end + 1;

        let last = &content[inside_end..].chars().next();

        match last {
            Some('"') => (),
            Some(_) | None => {
                return Err(ParseError::with_message(
                    "unterminated quoted string",
                    QuotedString::name(),
                    offset,
                    offset + end,
                ));
            }
        }

        let value = content[inside_start..inside_end].replace("\\\"", "\"");

        Ok((
            QuotedString {
                value,
                start: offset,
                end: end + offset,
            },
            end,
            None,
        ))
    }

    fn range(&self) -> (usize, usize) {
        (self.start, self.end)
    }

    fn name() -> &'static str {
        "quoted string"
    }
}

impl GrammarUnit for Whitespace {
    fn try_match(content: &str, offset: usize) -> Result<(Self, usize, Option<ParseError>)> {
        let size = take_char_while(content, char::is_whitespace);
        if size == 0 {
            return Err(ParseError::expected(Whitespace::name(), offset, offset + 1));
        }

        Ok((
            Whitespace {
                start: offset,
                end: offset + size,
            },
            size,
            None,
        ))
    }

    fn range(&self) -> (usize, usize) {
        (self.start, self.end)
    }

    fn name() -> &'static str {
        "whitespace"
    }
}

impl GrammarUnit for BareWord {
    fn try_match(content: &str, offset: usize) -> Result<(Self, usize, Option<ParseError>)> {
        let size = take_char_while(content, |c| char::is_alphanumeric(c) || c == '_');
        if size == 0 {
            return Err(ParseError::expected(Self::name(), offset, offset + 1));
        }

        Ok((
            BareWord {
                start: offset,
                end: offset + size,
            },
            size,
            None,
        ))
    }

    fn range(&self) -> (usize, usize) {
        (self.start, self.end)
    }

    fn name() -> &'static str {
        "bare word"
    }
}

impl GrammarUnit for Numeric {
    fn try_match(content: &str, offset: usize) -> Result<(Self, usize, Option<ParseError>)> {
        let mut size = if let Some(ch) = content.chars().next() {
            if ch == '+' || ch == '-' {
                1
            } else {
                0
            }
        } else {
            0
        };
        size += take_char_while(content, |c| {
            char::is_numeric(c) || c == '.' || c == 'e' || c == 'E'
        });

        if size == 0 {
            return Err(ParseError::expected(Self::name(), offset, offset + 1));
        }

        let value = match content[..size].parse::<f64>() {
            Ok(val) => val,
            Err(_) => {
                return Err(ParseError::with_message(
                    "unable to parse number",
                    Self::name(),
                    offset,
                    offset + size,
                ));
            }
        };

        Ok((
            Numeric {
                start: offset,
                end: offset + size,
                value,
            },
            size,
            None,
        ))
    }

    fn range(&self) -> (usize, usize) {
        (self.start, self.end)
    }

    fn name() -> &'static str {
        "number"
    }
}

impl GrammarUnit for Integer {
    fn try_match(content: &str, offset: usize) -> Result<(Self, usize, Option<ParseError>)> {
        let size = take_char_while(content, |c| char::is_numeric(c) || c == '-' || c == '+');
        if size == 0 {
            return Err(ParseError::expected(Self::name(), offset, offset + 1));
        }

        let value = match content[..size].parse::<i64>() {
            Ok(val) => val,
            Err(_) => {
                return Err(ParseError::with_message(
                    "unable to parse integer",
                    Self::name(),
                    offset,
                    offset + size,
                ));
            }
        };

        Ok((
            Integer {
                start: offset,
                end: offset + size,
                value,
            },
            size,
            None,
        ))
    }

    fn range(&self) -> (usize, usize) {
        (self.start, self.end)
    }

    fn name() -> &'static str {
        "integer"
    }
}

// Usually, identifiers can't start with a number, and otherwise must consist of alphanumeric
// characters plus underscore.
impl GrammarUnit for Identifier {
    fn try_match(mut content: &str, offset: usize) -> Result<(Self, usize, Option<ParseError>)> {
        match content.chars().next() {
            Some(ch) => {
                if !ch.is_alphabetic() && ch != '_' {
                    return Err(ParseError::with_message(
                        "identifiers must begin with a letter or underscore",
                        Self::name(),
                        offset,
                        offset + 1,
                    ));
                }
                content = &content[ch.len_utf8()..];
            }
            None => {
                return Err(ParseError::expected(Self::name(), offset, offset + 1));
            }
        }

        let size = 1 + take_char_while(content, |c| c.is_alphanumeric() || c == '_');
        if size == 0 {
            return Err(ParseError::expected(Self::name(), offset, offset + 1));
        }

        Ok((
            Self {
                start: offset,
                end: offset + size,
            },
            size,
            None,
        ))
    }

    fn range(&self) -> (usize, usize) {
        (self.start, self.end)
    }

    fn name() -> &'static str {
        "identifier"
    }
}

// Comment starts with two slashes and extends to a newline
impl GrammarUnit for Comment {
    fn try_match(mut content: &str, offset: usize) -> Result<(Self, usize, Option<ParseError>)> {
        if !content.starts_with("//") {
            return Err(ParseError::expected(Self::name(), offset, offset + 1));
        }

        let size = 2 + take_char_while(&content[2..], |c| c != '\n');
        Ok((
            Self {
                start: offset,
                end: offset + size,
            },
            size,
            None,
        ))
    }

    fn range(&self) -> (usize, usize) {
        (self.start, self.end)
    }

    fn name() -> &'static str {
        "comment"
    }
}

// Comment starts with two slashes and extends to a newline
impl GrammarUnit for EOF {
    fn try_match(mut content: &str, offset: usize) -> Result<(Self, usize, Option<ParseError>)> {
        if content.is_empty() {
            return Ok((Self { pos: offset }, 0, None));
        }

        Err(ParseError::expected(Self::name(), offset, offset + 1))
    }

    fn range(&self) -> (usize, usize) {
        (self.pos, self.pos)
    }

    fn name() -> &'static str {
        "EOF"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_range<G: GrammarUnit>(content: &str, expected: &str) {
        let result = match G::try_match(content, 0) {
            Ok((g, _, _)) => g,
            Err(_) => {
                if expected.is_empty() {
                    return;
                } else {
                    panic!("{} didn't match!", std::any::type_name::<G>());
                }
            }
        };
        let (start, end) = result.range();
        assert_eq!(
            expected,
            format!("{}{}", " ".repeat(start), "^".repeat(end - start),)
        );
    }

    #[test]
    fn test_quoted_string_match() {
        let (qs, took, _) = QuotedString::try_match(r#""hello, world" test"#, 0).unwrap();
        assert_eq!(took, 14);
        assert_eq!(&qs.value, "hello, world");

        assert!(QuotedString::try_match("", 0).is_err());
        let (qs, took, _) = QuotedString::try_match(r#""my ' string \" test""#, 0).unwrap();
        assert_eq!(took, 21);
        assert_eq!(&qs.value, "my ' string \" test");

        assert_range::<QuotedString>(
            r#""hello, world" test"#,
            r#"^^^^^^^^^^^^^^"#, // comment to prevent reformat
        )
    }

    #[test]
    fn test_whitespace_match() {
        let content = r#""hello, world"   test"#;
        let (_, took, _) = QuotedString::try_match(content, 0).unwrap();
        assert_eq!(took, 14);

        let remaining = &content[took..];

        let (_, took, _) = Whitespace::try_match(remaining, took).unwrap();

        assert_eq!(took, 3);

        assert_range::<Whitespace>(
            "     test",
            "^^^^^", // comment to prevent reformat
        )
    }

    #[test]
    fn test_identifier_match() {
        let content = r#"abc_def1"#;
        let (_, took, _) = Identifier::try_match(content, 0).unwrap();
        assert_eq!(took, 8);

        let content = r#"abc def1"#;
        let (_, took, _) = Identifier::try_match(content, 0).unwrap();
        assert_eq!(took, 3);

        let content = r#"_abc def1"#;
        let (_, took, _) = Identifier::try_match(content, 0).unwrap();
        assert_eq!(took, 4);

        let content = r#"1_abc def1"#;
        Identifier::try_match(content, 0).unwrap_err();
    }
}
