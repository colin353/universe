use ggen::{AtLeastOne, Comment, Identifier, Integer, EOF};

ggen::char_rule!(Whitespace, |ch: char| ch.is_whitespace() && ch != '\n');
ggen::char_rule!(Separator, |ch: char| ch.is_whitespace());

ggen::sequence!(
    Module,
    leading_comments: Vec<WhitespaceNewlineComment>,
    definitions: Vec<Definition>,
    trailing_comments: Vec<WhitespaceNewlineComment>,
    end: EOF,
);

ggen::one_of!(
    Definition,
    Message: MessageDefinition,
    Enum: EnumDefinition,
    Service: ServiceDefinition
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
    EnumDefinition,
    _leading_comments: Vec<WhitespaceNewlineComment>,
    "enum",
    _ws1: Option<Whitespace>,
    name: Identifier,
    _ws2: Vec<WhitespaceNewlineOrComment>,
    "{",
    _ws3: Vec<WhitespaceNewlineOrComment>,
    fields: Vec<EnumField>,
    _ws4: Vec<WhitespaceNewlineOrComment>,
    "}",
    _ws5: Option<WhitespaceNewlineComment>,
);

ggen::sequence!(
    EnumField,
    field_name: Identifier,
    _ws1: Option<Whitespace>,
    "=",
    _ws2: Option<Whitespace>,
    tag: Integer,
    _trailing_semicolon: Option<Semicolon>,
    _trailing_newline: AtLeastOne<WhitespaceNewlineComment>,
);

ggen::sequence!(
    MessageDefinition,
    _leading_comments: Vec<WhitespaceNewlineComment>,
    "message",
    _ws1: Option<Whitespace>,
    name: Identifier,
    _ws2: Vec<WhitespaceNewlineOrComment>,
    "{",
    _ws3: Vec<WhitespaceNewlineOrComment>,
    fields: Vec<FieldDefinition>,
    _ws4: Vec<WhitespaceNewlineOrComment>,
    "}",
    _ws5: Option<WhitespaceNewlineComment>,
);

ggen::unit!(Repeated, "repeated");
ggen::unit!(Stream, "stream");
ggen::unit!(Newline, "\n");
ggen::unit!(Semicolon, ";");

ggen::sequence!(
    FieldDefinition,
    field_name: Identifier,
    _ws1: Option<Whitespace>,
    ":",
    _ws2: Option<Whitespace>,
    repeated: Option<Repeated>,
    _ws3: Option<Whitespace>,
    type_name: Identifier,
    _ws4: Option<Whitespace>,
    "=",
    _ws5: Option<Whitespace>,
    tag: Integer,
    _trailing_semicolon: Option<Semicolon>,
    _trailing_newline: AtLeastOne<WhitespaceNewlineComment>,
);

ggen::sequence!(
    ServiceDefinition,
    _leading_comments: Vec<WhitespaceNewlineComment>,
    "service",
    _ws1: Option<Whitespace>,
    name: Identifier,
    _ws2: Vec<WhitespaceNewlineOrComment>,
    "{",
    _ws3: Vec<WhitespaceNewlineOrComment>,
    fields: Vec<RpcDefinition>,
    _ws4: Vec<WhitespaceNewlineOrComment>,
    "}",
    _ws5: Option<WhitespaceNewlineComment>,
);

ggen::sequence!(
    RpcDefinition,
    _ws1: Option<Whitespace>,
    "rpc",
    _ws2: Whitespace,
    name: Identifier,
    _ws3: Option<Whitespace>,
    "(",
    _ws4: Option<Whitespace>,
    argument_type: Identifier,
    _ws5: Option<Whitespace>,
    ")",
    _ws6: Option<Whitespace>,
    "->",
    _ws7: Option<Whitespace>,
    stream: Option<Stream>,
    _ws8: Option<Whitespace>,
    return_type: Identifier,
    _trailing_semicolon: Option<Semicolon>,
    _trailing_newline: AtLeastOne<WhitespaceNewlineComment>,
);

#[cfg(test)]
mod tests {
    use super::*;
    use ggen::GrammarUnit;

    #[test]
    fn test_field_definition_match() {
        FieldDefinition::try_match("age: repeated int32 = 1;\n", 0).unwrap();
        FieldDefinition::try_match("name: string = 2;\n", 0).unwrap();
        assert!(FieldDefinition::try_match("fake", 0).is_err());
    }

    #[test]
    fn test_message_definition_match() {
        MessageDefinition::try_match(
            r#"message MyMessage {
            name: string = 1;
        }"#,
            0,
        )
        .unwrap();

        // Empty message is valid
        MessageDefinition::try_match(r#"message Empty {}"#, 0).unwrap();

        MessageDefinition::try_match(
            r#"message Empty {
            z: repeated i32 = 1;
            query: repeated string = 2;
        }"#,
            0,
        )
        .unwrap();
    }

    #[test]
    fn test_service_definition_match() {
        ServiceDefinition::try_match(
            r#"service MyService {
    rpc read(ReadRequest) -> ReadResponse;
}"#,
            0,
        )
        .unwrap();
    }
}
