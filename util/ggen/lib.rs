mod basic;
mod macros;

pub use basic::{BareWord, QuotedString, Whitespace};

trait GrammarUnit: Sized + std::fmt::Debug {
    fn try_match(content: &str, offset: usize) -> Option<(Self, usize)>;
    fn range(&self) -> (usize, usize);
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
