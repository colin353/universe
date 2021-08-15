#[macro_export]
macro_rules! define_unit {
    ( $name:ident, $($term_name:ident: $term:ty,)* ; ) => {
        #[derive(Debug, PartialEq)]
        pub struct $name {
            $(
                $term_name: $term,
            )*
            _start: usize,
            _end: usize,
        }
    };
    ( $name:ident, $($tn:ident: $tt:ty,)* ; $value:literal, $($rest:tt)*) => {
        $crate::define_unit!($name, $($tn: $tt, )* ; $($rest)*);
    };
    ( $name:ident, $($tn:ident: $tt:ty,)* ; $term_name:ident: $term:ty, $($rest:tt)*) => {
        $crate::define_unit!($name, $($tn: $tt, )* $term_name: $term, ; $($rest)*);
    };
}

#[macro_export]
macro_rules! create_unit {
    ( $name:ident, $($term_name:ident: $term:ty,)* ; ) => {
        $name {
            $(
                $term_name,
            )*
            _start: 0,
            _end: 0,
        }
    };
    ( $name:ident, $($tn:ident: $tt:ty,)* ; $value:literal, $($rest:tt)*) => {
        $crate::create_unit!($name, $($tn: $tt, )* ; $($rest)*)
    };
    ( $name:ident, $($tn:ident: $tt:ty,)* ; $term_name:ident: $term:ty, $($rest:tt)*) => {
        $crate::create_unit!($name, $($tn: $tt, )* $term_name: $term, ; $($rest)*)
    };
}

#[macro_export]
macro_rules! impl_subunits {
    ( $remaining:expr, $taken:expr, $offset:expr, $term_name:ident: $term:ty, $($rest:tt)* ) => {
        let $term_name = match <$term>::try_match($remaining, $offset + $taken) {
            Some((t, took)) => {
                $taken += took;
                $remaining = &$remaining[took..];
                t
            }
            None => {
                return None
            },
        };
        $crate::impl_subunits!($remaining, $taken, $offset, $($rest)*);
    };
    ( $remaining:expr, $taken:expr, $offset:expr, $value:literal, $($rest:tt)* ) => {
        if $remaining.starts_with($value) {
            $taken += $value.len();
            $remaining = &$remaining[$value.len()..];
        } else {
            return None;
        }
        $crate::impl_subunits!($remaining, $taken, $offset, $($rest)*);
    };
    ( $remaining:expr, $taken:expr, $offset:expr, ) => {};
}

#[macro_export]
macro_rules! sequence {
    ( $name:ident, $( $args:tt )* ) => {
        $crate::define_unit!($name, ; $( $args )*);

        impl $crate::GrammarUnit for $name {
            fn try_match(content: &str, offset: usize) -> Option<(Self, usize)> {
                let mut taken = 0;
                let mut _remaining = content;

                $crate::impl_subunits!(_remaining, taken, offset, $( $args )*);

                let mut unit = $crate::create_unit!($name, ; $( $args )*);
                unit._start = offset;
                unit._end = taken + offset;

                Some((unit, taken))
            }

            fn range(&self) -> (usize, usize) {
                (self._start, self._end)
            }
        }
    };
}

#[macro_export]
macro_rules! one_of {
    ( $name:ident, $( $term_name:ident: $term:ty ),* ) => {
        #[derive(Debug, PartialEq)]
        pub enum $name {
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

#[macro_export]
macro_rules! unit {
    ( $name:ident, $value:literal ) => {
        #[derive(Debug, PartialEq)]
        pub struct $name {
            _start: usize,
            _end: usize,
        }

        impl $crate::GrammarUnit for $name {
            fn try_match(content: &str, offset: usize) -> Option<(Self, usize)> {
                if !content.starts_with($value) {
                    return None;
                }

                Some((
                    $name {
                        _start: offset,
                        _end: offset + $value.len(),
                    },
                    $value.len(),
                ))
            }

            fn range(&self) -> (usize, usize) {
                (self._start, self._end)
            }
        }
    };
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
            _ws2: Whitespace,
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

        assert_eq!(unit.string.value, "grammar");
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
            _suffix: Whitespace,
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
            _suffix: Option<Whitespace>,
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

    #[test]
    fn test_sequence_literal() {
        sequence!(
            Colin,
            _prefix: Option<Whitespace>,
            "colin",
            _suffix: Option<Whitespace>,
        );

        assert!(Colin::try_match("   colin   ", 0).is_some());
        assert!(Colin::try_match("   ballin   ", 0).is_none());
    }

    #[test]
    fn test_unit_literal() {
        unit!(BooleanTrue, "true");
        unit!(BooleanFalse, "false");

        one_of!(Boolean, True: BooleanTrue, False: BooleanFalse);

        assert!(Boolean::try_match("true", 0).is_some());
        assert!(Boolean::try_match("false", 0).is_some());
    }
}
