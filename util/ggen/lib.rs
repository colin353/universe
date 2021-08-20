mod basic;
mod macros;

pub use basic::{BareWord, Integer, Numeric, QuotedString, Whitespace};

pub type Result<T> = std::result::Result<T, ParseError>;

pub trait GrammarUnit: Sized + std::fmt::Debug {
    fn try_match(content: &str, offset: usize) -> Result<(Self, usize, Option<ParseError>)>;
    fn range(&self) -> (usize, usize);
    fn name() -> &'static str {
        std::any::type_name::<Self>()
    }
}

#[derive(Clone, Debug)]
pub struct ParseError {
    pub message: String,
    pub names: Vec<String>,
    pub start: usize,
    pub end: usize,
}

impl ParseError {
    pub fn new(message: String, name: &str, start: usize, end: usize) -> Self {
        Self {
            message,
            names: vec![name.to_owned()],
            start,
            end,
        }
    }

    pub fn new_multi_name(message: String, names: Vec<String>, start: usize, end: usize) -> Self {
        Self {
            message,
            names,
            start,
            end,
        }
    }

    pub fn render(&self, content: &str) -> String {
        let end = std::cmp::min(content.len(), self.end);
        let line_number = content[..end].lines().count();
        let line_start = content[..end].rfind('\n').map(|idx| idx + 1).unwrap_or(0);
        let line_content = content[line_start..].lines().next().unwrap_or("");

        let underline = " ".repeat(self.start - line_start) + &"^".repeat(self.end - self.start);

        format!(
            "   |\n{line_number:<3}| {line_content}\n   | {underline} {message}\n",
            line_number = line_number,
            line_content = line_content,
            underline = underline,
            message = self.message,
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
    pub inner: Vec<Unit>,
    _marker: std::marker::PhantomData<Separator>,
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
    pub fn new(inner: Vec<Unit>) -> Self {
        Self {
            inner,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
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
        (&self.inner).into_iter()
    }
}

impl<Unit: GrammarUnit, Separator: GrammarUnit> GrammarUnit
    for RepeatWithSeparator<Unit, Separator>
{
    fn try_match(content: &str, offset: usize) -> Result<(Self, usize, Option<ParseError>)> {
        let mut took = 0;
        let mut output = Vec::new();
        let mut seq_error = None;
        loop {
            match Unit::try_match(&content[took..], offset + took) {
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

            match Separator::try_match(&content[took..], offset + took) {
                Ok((_, t, seq_err)) => {
                    if let Some(seq_err) = seq_err {
                        seq_error = Some(seq_err.merge(seq_error));
                    }
                    took += t;
                }
                Err(err) => {
                    seq_error = Some(err.merge(seq_error));
                    break;
                }
            }
        }
        Ok((RepeatWithSeparator::new(output), took, seq_error))
    }

    fn range(&self) -> (usize, usize) {
        match (self.inner.first(), self.inner.last()) {
            (Some(first), Some(last)) => (first.range().0, last.range().1),
            (Some(x), None) | (None, Some(x)) => x.range(),
            (None, None) => (0, 0),
        }
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

fn take_char_while<F: Fn(char) -> bool>(content: &str, rule: F) -> usize {
    let mut position = 0;
    for ch in content.chars() {
        if !rule(ch) {
            break;
        }
        position += ch.len_utf8();
    }
    position
}
