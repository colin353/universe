mod basic;
mod macros;

pub use basic::{BareWord, Comment, Identifier, Integer, Numeric, QuotedString, Whitespace, EOF};

pub type Result<T> = std::result::Result<T, ParseError>;

pub trait GrammarUnit: Sized + std::fmt::Debug {
    fn try_match(content: &str, offset: usize) -> Result<(Self, usize, Option<ParseError>)>;
    fn range(&self) -> (usize, usize);
    fn name() -> &'static str {
        std::any::type_name::<Self>()
    }

    fn as_str<'a>(&self, content: &'a str) -> &'a str {
        let (start, end) = self.range();
        &content[start..end]
    }
}

#[derive(Clone, Debug)]
pub enum ParseErrorKind {
    Message(String),
    StaticMessage(&'static str),
    ExpectedOneOf,
    Expected,
}

#[derive(Clone, Debug)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub names: Vec<&'static str>,
    pub start: usize,
    pub end: usize,
}

impl ParseError {
    pub fn from_string(msg: String, name: &'static str, start: usize, end: usize) -> Self {
        Self {
            kind: ParseErrorKind::Message(msg),
            names: vec![name],
            start,
            end,
        }
    }

    pub fn with_message(msg: &'static str, name: &'static str, start: usize, end: usize) -> Self {
        Self {
            kind: ParseErrorKind::StaticMessage(msg),
            names: vec![name],
            start,
            end,
        }
    }

    pub fn expected(name: &'static str, start: usize, end: usize) -> Self {
        Self {
            kind: ParseErrorKind::Expected,
            names: vec![name],
            start,
            end,
        }
    }

    pub fn expected_one_of(names: Vec<&'static str>, start: usize, end: usize) -> Self {
        Self {
            kind: ParseErrorKind::ExpectedOneOf,
            names,
            start,
            end,
        }
    }

    pub fn render(&self, content: &str) -> String {
        let start = std::cmp::min(content.len(), self.start);
        let line_number = std::cmp::max(content[..start].lines().count(), 1);
        let line_start = content[..start].rfind('\n').map(|idx| idx + 1).unwrap_or(0);
        let line_content = content[line_start..].lines().next().unwrap_or("");

        let underline = " ".repeat(self.start - line_start) + &"^".repeat(self.end - self.start);

        let message = match &self.kind {
            ParseErrorKind::Expected => format!("expected {}", self.names.get(0).unwrap_or(&"??")),
            ParseErrorKind::ExpectedOneOf => format!("expected one of: {}", self.names.join(", ")),
            ParseErrorKind::StaticMessage(msg) => msg.to_string(),
            ParseErrorKind::Message(msg) => msg.clone(),
        };

        format!(
            "   |\n{line_number:<3}| {line_content}\n   | {underline} {message}\n",
            line_number = line_number,
            line_content = line_content,
            underline = underline,
            message = message,
        )
    }

    pub fn merge(&self, other: Option<ParseError>) -> ParseError {
        let other = match other.as_ref() {
            Some(x) => x,
            None => return self.clone(),
        };

        if other.end > self.end {
            return other.clone();
        }

        return self.clone();
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RepeatWithSeparator<Unit, Separator> {
    pub values: Vec<Unit>,
    pub separators: Vec<Separator>,
}

impl<U: GrammarUnit, S: GrammarUnit> RepeatWithSeparator<U, S> {
    pub fn empty() -> Self {
        Self {
            values: Vec::new(),
            separators: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AtLeastOne<Unit> {
    pub inner: Vec<Unit>,
}

impl<G: GrammarUnit> GrammarUnit for Option<G> {
    fn try_match(content: &str, offset: usize) -> Result<(Self, usize, Option<ParseError>)> {
        match G::try_match(content, offset) {
            Ok((unit, took, seq_err)) => Ok((Some(unit), took, seq_err)),
            Err(err) => Ok((None, 0, Some(err))),
        }
    }

    fn range(&self) -> (usize, usize) {
        match self {
            Some(x) => x.range(),
            None => (0, 0),
        }
    }
}

impl<G: GrammarUnit> GrammarUnit for Vec<G> {
    fn try_match(content: &str, offset: usize) -> Result<(Self, usize, Option<ParseError>)> {
        let mut took = 0;
        let mut output = Vec::new();
        let mut seq_error = None;
        loop {
            match G::try_match(&content[took..], offset + took) {
                Ok((unit, t, seq_err)) => {
                    took += t;
                    output.push(unit);
                    if let Some(seq_err) = seq_err {
                        seq_error = Some(seq_err.merge(seq_error));
                    }
                }
                Err(err) => {
                    seq_error = Some(err.merge(seq_error));
                    break;
                }
            }
        }
        Ok((output, took, seq_error))
    }

    fn range(&self) -> (usize, usize) {
        match (self.first(), self.last()) {
            (Some(first), Some(last)) => (first.range().0, last.range().1),
            (Some(x), None) | (None, Some(x)) => x.range(),
            (None, None) => (0, 0),
        }
    }

    fn name() -> &'static str {
        G::name()
    }
}

impl<Unit: GrammarUnit, Separator: GrammarUnit> RepeatWithSeparator<Unit, Separator> {
    pub fn new(values: Vec<Unit>, separators: Vec<Separator>) -> Self {
        Self { values, separators }
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Unit> {
        self.into_iter()
    }

    pub fn name() -> &'static str {
        Unit::name()
    }
}

impl<'a, U, S> IntoIterator for &'a RepeatWithSeparator<U, S> {
    type Item = &'a U;
    type IntoIter = std::slice::Iter<'a, U>;

    fn into_iter(self) -> Self::IntoIter {
        (&self.values).into_iter()
    }
}

impl<Unit: GrammarUnit, Separator: GrammarUnit> GrammarUnit
    for RepeatWithSeparator<Unit, Separator>
{
    fn try_match(content: &str, offset: usize) -> Result<(Self, usize, Option<ParseError>)> {
        let mut took = 0;
        let mut values = Vec::new();
        let mut separators = Vec::new();
        let mut seq_error = None;
        let mut sep_took = 0;
        loop {
            match Unit::try_match(&content[took..], offset + took) {
                Ok((unit, t, seq_err)) => {
                    took += t;
                    values.push(unit);
                    if let Some(seq_err) = seq_err {
                        seq_error = Some(seq_err.merge(seq_error));
                    }
                }
                Err(err) => {
                    if took == 0 {
                        return Err(err);
                    }

                    seq_error = Some(err.merge(seq_error));

                    // Revert the last separator
                    took -= sep_took;
                    separators.pop();
                    break;
                }
            }

            match Separator::try_match(&content[took..], offset + took) {
                Ok((sep, t, seq_err)) => {
                    if let Some(seq_err) = seq_err {
                        seq_error = Some(seq_err.merge(seq_error));
                    }
                    took += t;
                    sep_took = t;
                    separators.push(sep);
                }
                Err(err) => {
                    seq_error = Some(err.merge(seq_error));
                    break;
                }
            }
        }
        Ok((
            RepeatWithSeparator::new(values, separators),
            took,
            seq_error,
        ))
    }

    fn range(&self) -> (usize, usize) {
        match (self.values.first(), self.values.last()) {
            (Some(first), Some(last)) => (first.range().0, last.range().1),
            (Some(x), None) | (None, Some(x)) => x.range(),
            (None, None) => (0, 0),
        }
    }
}

impl<G: GrammarUnit> GrammarUnit for AtLeastOne<G> {
    fn try_match(content: &str, offset: usize) -> Result<(Self, usize, Option<ParseError>)> {
        let mut took = 0;
        let mut output = Vec::new();
        let mut seq_error = None;
        loop {
            match G::try_match(&content[took..], offset + took) {
                Ok((unit, t, seq_err)) => {
                    took += t;
                    output.push(unit);
                    if let Some(seq_err) = seq_err {
                        seq_error = Some(seq_err.merge(seq_error));
                    }
                }
                Err(err) => {
                    // Require at least one match
                    if took == 0 {
                        return Err(err);
                    }
                    seq_error = Some(err.merge(seq_error));
                    break;
                }
            }
        }
        Ok((Self { inner: output }, took, seq_error))
    }

    fn range(&self) -> (usize, usize) {
        match (self.inner.first(), self.inner.last()) {
            (Some(first), Some(last)) => (first.range().0, last.range().1),
            (Some(x), None) | (None, Some(x)) => x.range(),
            (None, None) => (0, 0),
        }
    }

    fn name() -> &'static str {
        G::name()
    }
}

fn take_while<F: Fn(&str) -> usize>(content: &str, rule: F) -> usize {
    let mut position = 0;
    loop {
        if position == content.len() {
            break;
        }
        let jump = rule(&content[position..]);
        if jump == 0 {
            break;
        }
        position += jump;
    }
    position
}

pub fn take_char_while<F: Fn(char) -> bool>(content: &str, rule: F) -> usize {
    let mut position = 0;
    for ch in content.chars() {
        if !rule(ch) {
            break;
        }
        position += ch.len_utf8();
    }
    position
}
