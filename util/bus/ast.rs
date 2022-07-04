use ggen::{AtLeastOne, Comment, Identifier, Integer, EOF};

ggen::char_rule!(Whitespace, |ch: char| ch.is_whitespace() && ch != '\n');
ggen::char_rule!(Separator, |ch: char| ch.is_whitespace());

ggen::sequence!(
    Module,
    leading_comments: Vec<WhitespaceNewlineComment>,
    definitions: Vec<MessageDefinition>,
    end: EOF,
);

ggen::sequence!(
    WhitespaceNewlineOrComment,
    _ws1: Separator,
    comment: Option<Comment>,
    _ws2: Option<Separator>,
);

ggen::sequence!(
    WhitespaceNewlineComment,
    _ws1: Option<Whitespace>,
    comment: Option<Comment>,
    "\n",
    _ws2: Option<Whitespace>,
);

ggen::sequence!(
    MessageDefinition,
    "message",
    _ws1: Option<Whitespace>,
    name: Identifier,
    _ws2: Vec<WhitespaceNewlineOrComment>,
    "{",
    _ws3: Vec<WhitespaceNewlineOrComment>,
    fields: Vec<FieldDefinition>,
    _ws4: Vec<WhitespaceNewlineOrComment>,
    "}",
    _ws5: Vec<WhitespaceNewlineOrComment>,
);

ggen::unit!(Repeated, "repeated");
ggen::unit!(Newline, "\n");
ggen::unit!(Semicolon, ";");

ggen::sequence!(
    FieldDefinition,
    repeated: Option<Repeated>,
    _ws1: Option<Whitespace>,
    type_name: Identifier,
    _ws2: Option<Whitespace>,
    field_name: Identifier,
    _ws3: Option<Whitespace>,
    "=",
    _ws4: Option<Whitespace>,
    tag: Integer,
    _trailing_semicolon: Option<Semicolon>,
    _trailing_newline: AtLeastOne<WhitespaceNewlineComment>,
);

#[cfg(test)]
mod tests {
    use super::*;
    use ggen::GrammarUnit;

    #[test]
    fn test_field_definition_match() {
        FieldDefinition::try_match("repeated int32 age = 1;\n", 0).unwrap();
        FieldDefinition::try_match("string name = 2;\n", 0).unwrap();
        assert!(FieldDefinition::try_match("fake", 0).is_err());
    }

    #[test]
    fn test_message_definition_match() {
        MessageDefinition::try_match(
            r#"message MyMessage {
            string name = 1;
        }"#,
            0,
        )
        .unwrap();

        // Empty message is valid
        MessageDefinition::try_match(r#"message Empty {}"#, 0).unwrap();

        MessageDefinition::try_match(
            r#"message Empty {
            repeated int32 z = 1;
            repeated string query = 2;
        }"#,
            0,
        )
        .unwrap();
    }
}
