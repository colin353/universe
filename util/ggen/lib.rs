mod basic;
mod macros;

pub use basic::{BareWord, QuotedString, Whitespace};

trait GrammarUnit: Sized + std::fmt::Debug {
    fn try_match(content: &str, offset: usize) -> Option<(Self, usize)>;
    fn range(&self) -> (usize, usize);
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
        while let Some((unit, t)) = G::try_match(content, offset) {
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
