#[macro_export]
macro_rules! define_unit {
    ( $name:ident, $($term_name:ident: $term:ty,)* ; ) => {
        #[derive(Clone, Debug, PartialEq)]
        pub struct $name {
            $(
               pub $term_name: $term,
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
    ( $remaining:expr, $taken:expr, $offset:expr, $seq_error:expr, $term_name:ident: $term:ty, $($rest:tt)* ) => {
        let $term_name = match <$term>::try_match($remaining, $offset + $taken) {
            Ok((t, took, seq_err)) => {
                $taken += took;
                $remaining = &$remaining[took..];
                if let Some(this_seq_err) = seq_err {
                    if let Some(existing_seq_err) = $seq_error.as_ref() {
                        if this_seq_err.end > existing_seq_err.end {
                            $seq_error = Some(this_seq_err);
                        }
                    } else {
                        $seq_error = Some(this_seq_err);
                    }
                }
                t
            }
            Err(e) => {
                if let Some(seq_err) = $seq_error {
                    if seq_err.end > e.end {
                        return Err(seq_err);
                    } else if seq_err.end == e.end {
                        let mut names: Vec<&'static str> = seq_err.names;
                        names.extend(&e.names);
                        return Err($crate::ParseError::expected_one_of(
                            names,
                            e.end,
                            e.end + 1,
                        ));
                    }
                }
                return Err(e);
            },
        };
        $crate::impl_subunits!($remaining, $taken, $offset, $seq_error, $($rest)*);
    };
    ( $remaining:expr, $taken:expr, $offset:expr, $seq_error:expr, $value:literal, $($rest:tt)* ) => {
        if $remaining.starts_with($value) {
            $taken += $value.len();
            $remaining = &$remaining[$value.len()..];
        } else {
            if let Some(seq_err) = $seq_error {
                if seq_err.end > $offset + $taken + 1 {
                    return Err(seq_err);
                } else if seq_err.end == $offset + $taken + 1 {
                    let mut names: Vec<&'static str> = seq_err.names;

                    match $value {
                        "\n" => names.push("newline"),
                        "\t" => names.push("tab"),
                        " " => names.push("space"),
                        x => names.push(x),
                    }

                    return Err($crate::ParseError::expected_one_of(
                        names,
                        $offset + $taken,
                        $offset + $taken + 1,
                    ));
                }
            }

            return Err($crate::ParseError::expected(
                $value,
                $offset + $taken,
                $offset + $taken + 1,
            ));
        }
        $crate::impl_subunits!($remaining, $taken, $offset, $seq_error, $($rest)*);
    };
    ( $remaining:expr, $taken:expr, $offset:expr, $seq_error:expr, ) => { };
}

#[macro_export]
macro_rules! sequence {
    ( $name:ident, $( $args:tt )* ) => {
        $crate::define_unit!($name, ; $( $args )*);

        impl $crate::GrammarUnit for $name {
            fn try_match(content: &str, offset: usize) -> $crate::Result<(Self, usize, Option<$crate::ParseError>)> {
                let mut taken = 0;
                let mut _remaining = content;
                let mut seq_error: Option<$crate::ParseError> = None;

                $crate::impl_subunits!(_remaining, taken, offset, seq_error, $( $args )*);

                let mut unit = $crate::create_unit!($name, ; $( $args )*);
                unit._start = offset;
                unit._end = taken + offset;

                Ok((unit, taken, seq_error))
            }

            fn range(&self) -> (usize, usize) {
                (self._start, self._end)
            }

            fn name() -> &'static str {
                stringify!($name)
            }
        }
    };
}

#[macro_export]
macro_rules! one_of {
    ( $name:ident, $( $term_name:ident: $term:ty ),* ) => {
        #[derive(Clone, Debug, PartialEq)]
        pub enum $name {
            $(
                $term_name(Box<$term>),
            )*
        }

        impl $crate::GrammarUnit for $name {
            fn try_match(content: &str, offset: usize) -> $crate::Result<(Self, usize, Option<$crate::ParseError>)> {
                let mut progress = 0;
                let mut unmatched = Vec::new();
                let mut error: Option<$crate::ParseError> = None;

                $(
                    match <$term>::try_match(content, offset) {
                        Ok((unit, took, seq_err)) => {
                            if let Some(this_seq_err) = seq_err {
                                if let Some(existing_seq_err) = error.as_ref() {
                                    if this_seq_err.end > existing_seq_err.end {
                                        error = Some(this_seq_err.clone());
                                    }
                                } else {
                                    error = Some(this_seq_err.clone());
                                }

                                let took = this_seq_err.end - offset;
                                if took > progress {
                                    unmatched = this_seq_err.names.clone();
                                    error = Some(this_seq_err.clone());
                                    progress = took;
                                } else if took == progress {
                                    for name in &this_seq_err.names {
                                        unmatched.push(name);
                                    }
                                }
                            }
                            return Ok(($name::$term_name(Box::new(unit)), took, error))
                        },
                        Err(err) => {
                            if err.end < offset {
                                panic!("malformed error range from {}", stringify!($term));
                            }
                            let took = err.end - offset;
                            if took > progress {
                                unmatched = vec![<$term>::name()];
                                error = Some(err);
                                progress = took;
                            } else if took == progress {
                                unmatched.push(<$term>::name());
                            }
                        }
                    }
                )*

                if unmatched.len() == 1 {
                    return Err(error.expect("error was not set!"));
                } else {
                    return Err($crate::ParseError::expected_one_of(
                        unmatched,
                        offset + progress - 1,
                        offset + progress,
                    ));
                }
            }

            fn range(&self) -> (usize, usize) {
                match self {
                    $(
                        Self::$term_name(x) => x.range(),
                    )*
                }
            }

            fn name() -> &'static str {
                stringify!($name)
            }
        }
    }
}

#[macro_export]
macro_rules! unit {
    ( $name:ident, $value:literal ) => {
        #[derive(Clone, Debug, PartialEq)]
        pub struct $name {
            _start: usize,
            _end: usize,
        }

        impl $crate::GrammarUnit for $name {
            fn try_match(
                content: &str,
                offset: usize,
            ) -> $crate::Result<(Self, usize, Option<$crate::ParseError>)> {
                if !content.starts_with($value) {
                    return Err($crate::ParseError::expected(
                        <$name>::name(),
                        offset,
                        offset + 1,
                    ));
                }

                Ok((
                    $name {
                        _start: offset,
                        _end: offset + $value.len(),
                    },
                    $value.len(),
                    None,
                ))
            }

            fn range(&self) -> (usize, usize) {
                (self._start, self._end)
            }

            fn name() -> &'static str {
                match $value {
                    "." => "\".\"",
                    "," => "\",\"",
                    "\n" => "newline",
                    "\t" => "tab",
                    x => x,
                }
            }
        }
    };
}

#[macro_export]
macro_rules! char_rule {
    ( $name:ident, $rule:expr ) => {
        #[derive(Clone, Debug, PartialEq)]
        pub struct $name {
            _start: usize,
            _end: usize,
        }

        impl $crate::GrammarUnit for $name {
            fn try_match(
                content: &str,
                offset: usize,
            ) -> $crate::Result<(Self, usize, Option<$crate::ParseError>)> {
                let size = $crate::take_char_while(content, $rule);

                if size == 0 {
                    return Err($crate::ParseError::expected(
                        Self::name(),
                        offset,
                        offset + 1,
                    ));
                }

                Ok((
                    $name {
                        _start: offset,
                        _end: offset + size,
                    },
                    size,
                    None,
                ))
            }

            fn range(&self) -> (usize, usize) {
                (self._start, self._end)
            }

            fn name() -> &'static str {
                stringify!($name)
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

    macro_rules! assert_fail {
        ($unit:ty, $content:expr, $expected:expr,) => {
            let fail = <$unit>::try_match($content, 0);
            assert!(fail.is_err());
            let got = fail.unwrap_err().render($content);
            if got.trim() != $expected.trim() {
                println!("got:\n\n{}\n", got.trim_matches('\n'));
                println!("expected:\n\n{}\n", $expected.trim_matches('\n'));
                panic!("got != expected");
            }
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

        let (unit, _, _) = StringWithWhitespace::try_match(r#"    "grammar"  "#, 0).unwrap();

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

        let (unit, _, _) = StringOrWhitespace::try_match("   xyz", 0).unwrap();

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

        let (unit, _, _) = PaddedTerm::try_match("   xyz  ", 0).unwrap();
        assert_range!(
            &unit.term, //
            "   xyz",   //
            "   ^^^",
        );

        let (unit, _, _) = PaddedTerm::try_match(r#"   "term"  "#, 0).unwrap();
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

        let (unit, _, _) = MaybePaddedTerm::try_match("xyz", 0).unwrap();
        assert_range!(
            &unit.term, //
            "xyz",      //
            "^^^",
        );

        let (unit, _, _) = MaybePaddedTerm::try_match("   xyz", 0).unwrap();
        assert_range!(
            &unit.term, //
            "   xyz",   //
            "   ^^^",
        );

        let (unit, _, _) = MaybePaddedTerm::try_match("xyz   ", 0).unwrap();
        assert_range!(
            &unit.term, //
            "xyz   ",   //
            "^^^",
        );

        let (unit, _, _) = MaybePaddedTerm::try_match("   xyz   ", 0).unwrap();
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

        assert!(Colin::try_match("   colin   ", 0).is_ok());

        assert_fail!(
            Colin,
            "   ballin   ",
            r#"
   |
1  |    ballin   
   |    ^ expected colin
"#,
        );
    }

    #[test]
    fn test_unit_literal() {
        unit!(BooleanTrue, "true");
        unit!(BooleanFalse, "false");

        one_of!(Boolean, True: BooleanTrue, False: BooleanFalse);

        assert!(Boolean::try_match("true", 0).is_ok());
        assert!(Boolean::try_match("false", 0).is_ok());
        assert_fail!(
            Boolean,
            "groose",
            r#"
   |
1  | groose
   | ^ expected one of: true, false
"#,
        );
    }

    #[test]
    fn test_one_of_failure() {
        one_of!(Term, QuotedString: QuotedString, BareWord: BareWord);
        assert_fail!(
            Term,
            "#",
            r#"
   |
1  | #
   | ^ expected one of: quoted string, bare word
"#,
        );

        assert_fail!(
            Term,
            r#""groose"#,
            r#"
   |
1  | "groose
   | ^^^^^^^^ unterminated quoted string
"#,
        );
    }

    #[test]
    fn test_seq_errors() {
        let (_, _, maybe_seq_err) = Vec::<QuotedString>::try_match(r#""abcdef""ssss"#, 0).unwrap();
        let seq_err = maybe_seq_err.unwrap();
        assert_eq!(seq_err.start, 8);
        assert_eq!(seq_err.end, 14);

        sequence!(QuotedStringNewline, strings: Vec<QuotedString>, "\n",);

        let seq_err = QuotedStringNewline::try_match(r#""abcdef""ssss"#, 0).unwrap_err();
        assert_eq!(seq_err.start, 8);
        assert_eq!(seq_err.end, 14);
    }

    #[test]
    fn test_from_char_rule() {
        char_rule!(Word, char::is_alphabetic);
        let (unit, _, _) = Word::try_match("hello world", 0).unwrap();
        assert_range!(
            &unit,         //
            "hello world", //
            "^^^^^",
        );

        // Shouldn't match
        Word::try_match("1", 0).unwrap_err();
    }
}
