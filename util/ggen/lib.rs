mod basic;
mod macros;

pub use basic::{BareWord, Integer, Numeric, QuotedString, Whitespace};

pub trait GrammarUnit: Sized + std::fmt::Debug {
    fn try_match(content: &str, offset: usize) -> Option<(Self, usize)>;
    fn range(&self) -> (usize, usize);
}

#[derive(Debug, Clone, PartialEq)]
pub struct RepeatWithSeparator<Unit, Separator> {
    pub inner: Vec<Unit>,
    _marker: std::marker::PhantomData<Separator>,
}

impl<G: GrammarUnit> GrammarUnit for Option<G> {
    fn try_match(content: &str, offset: usize) -> Option<(Self, usize)> {
        match G::try_match(content, offset) {
            Some((unit, took)) => Some((Some(unit), took)),
            None => Some((None, 0)),
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
    fn try_match(content: &str, offset: usize) -> Option<(Self, usize)> {
        let mut took = 0;
        let mut output = Vec::new();
        while let Some((unit, t)) = G::try_match(&content[took..], offset + took) {
            took += t;
            output.push(unit);
        }
        Some((output, took))
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
    fn try_match(content: &str, offset: usize) -> Option<(Self, usize)> {
        let mut took = 0;
        let mut output = Vec::new();
        while let Some((unit, t)) = Unit::try_match(&content[took..], offset + took) {
            took += t;
            output.push(unit);

            if let Some((sep, t)) = Separator::try_match(&content[took..], offset + took) {
                took += t;
            } else {
                break;
            }
        }
        Some((RepeatWithSeparator::new(output), took))
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
