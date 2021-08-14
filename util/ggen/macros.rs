macro_rules! sequence {
    ( $name:ident, $( $term_name:ident: $term:ty ),* ) => {
        #[derive(Debug)]
        struct $name {
            $(
                $term_name: $term,
            )*
            _start: usize,
            _end: usize,
        }

        impl $crate::GrammarUnit for $name {
            fn try_match(content: &str, offset: usize) -> Option<(Self, usize)> {
                let mut taken = 0;
                let _remaining = content;

                $(
                    let $term_name = match <$term>::try_match(_remaining, offset + taken) {
                        Some((t, took)) => {
                            taken += took;
                            t
                        },
                        None => return None,
                    };

                    let _remaining = &content[taken..];
                )*

                Some(($name{
                    $(
                        $term_name,
                    )*
                    _start: offset,
                    _end: taken + offset,
                }, taken))
            }

            fn range(&self) -> (usize, usize) {
                (self._start, self._end)
            }
        }
    };
}

macro_rules! one_of {
    ( $name:ident, $( $term_name:ident: $term:ty ),* ) => {
        #[derive(Debug)]
        enum $name {
            $(
                $term_name($term),
            )*
        }

        impl $crate::GrammarUnit for $name {
            fn try_match(content: &str, offset: usize) -> Option<(Self, usize)> {
                $(
                    if let Some((unit, took)) = <$term>::try_match(content, offset) {
                        return Some(($name::$term_name(unit), took));
                    }
                )*

                return None;
            }

            fn range(&self) -> (usize, usize) {
                match self {
                    $(
                        Self::$term_name(x) => x.range(),
                    )*
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{BareWord, GrammarUnit, QuotedString, Whitespace};

    macro_rules! assert_range {
        ($unit:expr, $content:expr, $expected:expr,) => {
            let (start, end) = $unit.range();
            assert_eq!(
                $expected,
                format!("{}{}", " ".repeat(start), "^".repeat(end - start),)
            );
        };
    }

    #[test]
    fn test_sequence() {
        sequence!(
            StringWithWhitespace,
            _ws1: Whitespace,
            string: QuotedString,
            _ws2: Whitespace
        );

        let (unit, _) = StringWithWhitespace::try_match(r#"    "grammar"  "#, 0).unwrap();

        assert_range!(
            &unit,
            r#"    "grammar"  "#,
            r#"^^^^^^^^^^^^^^^"#, // comment to keep formatting
        );

        assert_range!(
            &unit.string,
            r#"    "grammar"  "#,
            r#"    ^^^^^^^^^"#, // comment to keep formatting
        );

        assert_eq!(unit.string.inner, "grammar");
    }

    #[test]
    fn test_one_of() {
        one_of!(
            StringOrWhitespace,
            QuotedString: QuotedString,
            Whitespace: Whitespace
        );

        let (unit, _) = StringOrWhitespace::try_match("   xyz", 0).unwrap();

        assert_range!(
            &unit,    //
            "   xyz", //
            "^^^",
        );
    }

    #[test]
    fn test_combinators() {
        one_of!(Term, QuotedString: QuotedString, BareWord: BareWord);
        sequence!(
            PaddedTerm,
            _prefix: Whitespace,
            term: Term,
            _suffix: Whitespace
        );

        let (unit, _) = PaddedTerm::try_match("   xyz  ", 0).unwrap();
        assert_range!(
            &unit.term, //
            "   xyz",   //
            "   ^^^",
        );

        let (unit, _) = PaddedTerm::try_match(r#"   "term"  "#, 0).unwrap();
        assert_range!(
            &unit.term, //
            r#"   "term"  "#,
            r#"   ^^^^^^"#,
        );
    }

    #[test]
    fn test_optional() {
        one_of!(Term, QuotedString: QuotedString, BareWord: BareWord);
        sequence!(
            MaybePaddedTerm,
            _prefix: Option<Whitespace>,
            term: Term,
            _suffix: Option<Whitespace>
        );

        let (unit, _) = MaybePaddedTerm::try_match("xyz", 0).unwrap();
        assert_range!(
            &unit.term, //
            "xyz",      //
            "^^^",
        );

        let (unit, _) = MaybePaddedTerm::try_match("   xyz", 0).unwrap();
        assert_range!(
            &unit.term, //
            "   xyz",   //
            "   ^^^",
        );

        let (unit, _) = MaybePaddedTerm::try_match("xyz   ", 0).unwrap();
        assert_range!(
            &unit.term, //
            "xyz   ",   //
            "^^^",
        );

        let (unit, _) = MaybePaddedTerm::try_match("   xyz   ", 0).unwrap();
        assert_range!(
            &unit.term,  //
            "   xyz   ", //
            "   ^^^",
        );
    }
}
