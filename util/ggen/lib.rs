mod basic;
mod macros;

pub use basic::{BareWord, Integer, Numeric, QuotedString, Whitespace};

pub type Result<T> = std::result::Result<T, ParseError>;

pub trait GrammarUnit: Sized + std::fmt::Debug {
    fn try_match(content: &str, offset: usize) -> Result<(Self, usize)>;
    fn range(&self) -> (usize, usize);
}

#[derive(Debug)]
pub struct ParseError {
    message: String,
    start: usize,
    end: usize,
}

impl ParseError {
    pub fn new(message: String, start: usize, end: usize) -> Self {
        Self {
            message,
            start,
            end,
        }
    }

    pub fn render(&self, content: &str) -> String {
        let line_number = content[..self.end].lines().count();
        let line_start = content[..self.end]
            .rfind('\n')
            .map(|idx| idx + 1)
            .unwrap_or(0);
        let line_content = content[line_start..].lines().next().unwrap_or("");

        let underline = " ".repeat(self.start - line_start) + &"^".repeat(self.end - self.start);

        format!(
            "   |\n{line_number:<3}|{line_content}\n   |{underline} {message}\n",
            line_number = line_number,
            line_content = line_content,
            underline = underline,
            message = self.message,
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RepeatWithSeparator<Unit, Separator> {
    pub inner: Vec<Unit>,
    _marker: std::marker::PhantomData<Separator>,
}

impl<G: GrammarUnit> GrammarUnit for Option<G> {
    fn try_match(content: &str, offset: usize) -> Result<(Self, usize)> {
        match G::try_match(content, offset) {
            Ok((unit, took)) => Ok((Some(unit), took)),
            Err(_) => Ok((None, 0)),
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
    fn try_match(content: &str, offset: usize) -> Result<(Self, usize)> {
        let mut took = 0;
        let mut output = Vec::new();
        while let Ok((unit, t)) = G::try_match(&content[took..], offset + took) {
            took += t;
            output.push(unit);
        }
        Ok((output, took))
    }

    fn range(&self) -> (usize, usize) {
        match (self.first(), self.last()) {
            (Some(first), Some(last)) => (first.range().0, last.range().1),
            (Some(x), None) | (None, Some(x)) => x.range(),
            (None, None) => (0, 0),
        }
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
    fn try_match(content: &str, offset: usize) -> Result<(Self, usize)> {
        let mut took = 0;
        let mut output = Vec::new();
        while let Ok((unit, t)) = Unit::try_match(&content[took..], offset + took) {
            took += t;
            output.push(unit);

            if let Ok((_, t)) = Separator::try_match(&content[took..], offset + took) {
                took += t;
            } else {
                break;
            }
        }
        Ok((RepeatWithSeparator::new(output), took))
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
