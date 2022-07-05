use ggen::GrammarUnit;

use std::collections::HashSet;

mod ast;

#[derive(Clone, Debug)]
pub enum CarError {
    ParseError(ggen::ParseError),
}

pub struct Module {
    pub messages: Vec<MessageDefinition>,
    pub enums: Vec<EnumDefinition>,
}

pub struct MessageDefinition {
    pub name: String,
    pub fields: Vec<FieldDefinition>,
    pub ast: ast::MessageDefinition,
}

pub struct FieldDefinition {
    pub repeated: bool,
    pub field_type: FieldType,
    pub field_name: String,
    pub tag: u32,
    pub ast: ast::FieldDefinition,
}

pub struct EnumDefinition {
    pub name: String,
    pub fields: Vec<(String, u8)>,
    pub ast: ast::EnumDefinition,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FieldType {
    Tu64,
    Tu32,
    Tu16,
    Tu8,
    Tbool,
    Tstring,
    Tfloat,
    Tbytes,
    Other(String),
}

impl FieldType {
    fn from(s: &str) -> Self {
        match s {
            "u64" => Self::Tu64,
            "u32" => Self::Tu32,
            "u16" => Self::Tu16,
            "u8" => Self::Tu8,
            "bool" => Self::Tbool,
            "string" => Self::Tstring,
            "float" => Self::Tfloat,
            "bytes" => Self::Tbytes,
            _ => Self::Other(s.to_owned()),
        }
    }
}

fn convert_field(f: &ast::FieldDefinition, data: &str) -> Result<FieldDefinition, CarError> {
    if f.tag.value < 0 {
        let (start, end) = f.tag.range();
        return Err(CarError::ParseError(ggen::ParseError::from_string(
            String::from("field numbers must be greater than zero"),
            "",
            start,
            end,
        )));
    }

    Ok(FieldDefinition {
        repeated: f.repeated.is_some(),
        field_type: FieldType::from(f.type_name.as_str(data)),
        field_name: f.field_name.as_str(data).to_owned(),
        tag: f.tag.value as u32,
        ast: f.clone(),
    })
}

fn convert_enum(e: ast::EnumDefinition, data: &str) -> Result<EnumDefinition, CarError> {
    let mut fields = Vec::new();
    let mut names = HashSet::new();
    for f in e.fields.iter() {
        let name = f.field_name.as_str(data).to_owned();
        if f.tag.value < 0 {
            let (start, end) = f.tag.range();
            return Err(CarError::ParseError(ggen::ParseError::from_string(
                String::from("field numbers must be greater than zero"),
                "",
                start,
                end,
            )));
        }

        if f.tag.value == 0 && name.as_str() != "Unknown" {
            let (start, end) = f.tag.range();
            return Err(CarError::ParseError(ggen::ParseError::from_string(
                String::from("the zero enum value must be called `Unknown`"),
                "",
                start,
                end,
            )));
        }

        if f.tag.value > 255 {
            let (start, end) = f.tag.range();
            return Err(CarError::ParseError(ggen::ParseError::from_string(
                String::from("a maximum of 255 values are allowed in an enum"),
                "",
                start,
                end,
            )));
        }

        if names.insert(name.clone()) {
            fields.push((name, f.tag.value as u8));
        } else {
            let (start, end) = f.range();
            return Err(CarError::ParseError(ggen::ParseError::from_string(
                format!("a field named `{}` already exists in this message", name,),
                "",
                start,
                end,
            )));
        }
    }
    Ok(EnumDefinition {
        name: e.name.as_str(data).to_owned(),
        fields,
        ast: e.clone(),
    })
}

fn convert_message(msg: ast::MessageDefinition, data: &str) -> Result<MessageDefinition, CarError> {
    let mut fields = Vec::new();
    let mut names = HashSet::new();
    for f in msg.fields.iter() {
        let cf = convert_field(f, data)?;
        if names.insert(cf.field_name.clone()) {
            fields.push(cf);
        } else {
            let (start, end) = f.range();
            return Err(CarError::ParseError(ggen::ParseError::from_string(
                format!(
                    "a field named `{}` already exists in this message",
                    cf.field_name
                ),
                "",
                start,
                end,
            )));
        }
    }
    Ok(MessageDefinition {
        name: msg.name.as_str(data).to_owned(),
        fields,
        ast: msg.clone(),
    })
}

pub fn parse(data: &str) -> Result<Module, CarError> {
    let (module, _, _) = ast::Module::try_match(data, 0).map_err(|e| CarError::ParseError(e))?;

    let mut messages = Vec::new();
    let mut enums = Vec::new();
    let mut types = HashSet::new();
    for d in module.definitions.into_iter() {
        match d {
            ast::Definition::Message(msg) => {
                let m = convert_message(*msg, data)?;

                if types.insert(m.name.clone()) {
                    messages.push(m);
                } else {
                    let (start, end) = m.ast.name.range();
                    return Err(CarError::ParseError(ggen::ParseError::from_string(
                        format!("the name `{}` already exists", m.name),
                        "",
                        start,
                        end,
                    )));
                }
            }
            ast::Definition::Enum(e) => {
                let e = convert_enum(*e, data)?;
                if types.insert(e.name.clone()) {
                    enums.push(e);
                } else {
                    let (start, end) = e.ast.name.range();
                    return Err(CarError::ParseError(ggen::ParseError::from_string(
                        format!("the name `{}` already exists", e.name),
                        "",
                        start,
                        end,
                    )));
                }
            }
        }
    }

    // Validate that all types are resolved
    for msg in &messages {
        for field in &msg.fields {
            if let FieldType::Other(s) = &field.field_type {
                if !types.contains(s) {
                    let (start, end) = field.ast.type_name.range();
                    return Err(CarError::ParseError(ggen::ParseError::from_string(
                        format!("unrecognized field type `{}`", &s),
                        "",
                        start,
                        end,
                    )));
                }
            }
        }
    }

    Ok(Module { messages, enums })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_module() {
        let content = r#"
message Something {
    size: repeated u32 = 1;
}
        "#;
        let module = parse(content).unwrap();
        assert_eq!(module.messages.len(), 1);
        assert_eq!(module.messages[0].fields.len(), 1);
        assert_eq!(module.messages[0].fields[0].repeated, true);
        assert_eq!(module.messages[0].fields[0].field_type, FieldType::Tu32);
        assert_eq!(&module.messages[0].fields[0].field_name, "size");
        assert_eq!(module.messages[0].fields[0].tag, 1);
    }

    #[test]
    fn test_parse_enums() {
        let content = r#"
enum Something {
    Unknown = 0
    Basic = 1
    Advanced = 2
}
        "#;
        let module = parse(content).unwrap();
        assert_eq!(module.enums.len(), 1);
        assert_eq!(module.enums[0].fields.len(), 3);
        assert_eq!(&module.enums[0].fields[0].0, "Unknown");
        assert_eq!(module.enums[0].fields[0].1, 0);
        assert_eq!(&module.enums[0].fields[1].0, "Basic");
        assert_eq!(module.enums[0].fields[1].1, 1);
    }
}
