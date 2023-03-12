use ggen::GrammarUnit;

use std::collections::{HashMap, HashSet};

pub mod ast;

#[derive(Clone, Debug)]
pub enum BusError {
    ParseError(ggen::ParseError),
}

pub struct Module {
    pub messages: Vec<MessageDefinition>,
    pub enums: Vec<EnumDefinition>,
    pub services: Vec<ServiceDefinition>,
}

pub struct ServiceDefinition {
    pub name: String,
    pub ast: ast::ServiceDefinition,
    pub rpcs: Vec<RpcDefinition>,
}

pub struct RpcDefinition {
    pub name: String,
    pub argument_type: String,
    pub return_type: String,
    pub ast: ast::RpcDefinition,
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
    Ti64,
    Ti32,
    Ti16,
    Ti8,
    Tu64,
    Tu32,
    Tu16,
    Tu8,
    Tbool,
    Tstring,
    Tfloat,
    Tbytes,
    Message(String),
    Enum(String),
    Other(String),
}

impl FieldType {
    fn from(s: &str) -> Self {
        match s {
            "i64" => Self::Ti64,
            "i32" => Self::Ti32,
            "i16" => Self::Ti16,
            "i8" => Self::Ti8,
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

fn convert_field(f: &ast::FieldDefinition, data: &str) -> Result<FieldDefinition, BusError> {
    if f.tag.value < 0 {
        let (start, end) = f.tag.range();
        return Err(BusError::ParseError(ggen::ParseError::from_string(
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

fn allowed_default_name(value: &str) -> bool {
    match value {
        "None" | "Unknown" | "Disabled" | "Default" => true,
        _ => false,
    }
}

fn convert_enum(e: ast::EnumDefinition, data: &str) -> Result<EnumDefinition, BusError> {
    let mut fields = Vec::new();
    let mut names = HashSet::new();
    for f in e.fields.iter() {
        let name = f.field_name.as_str(data).to_owned();
        if f.tag.value < 0 {
            let (start, end) = f.tag.range();
            return Err(BusError::ParseError(ggen::ParseError::from_string(
                String::from("field numbers must be greater than zero"),
                "",
                start,
                end,
            )));
        }

        if f.tag.value == 0 && !allowed_default_name(&name) {
            let (start, end) = f.field_name.range();
            return Err(BusError::ParseError(ggen::ParseError::from_string(
                String::from(
                    "the zero enum value must be called `Unknown`, `None`, `Disabled` or `Default`",
                ),
                "",
                start,
                end,
            )));
        }

        if f.tag.value > 255 {
            let (start, end) = f.tag.range();
            return Err(BusError::ParseError(ggen::ParseError::from_string(
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
            return Err(BusError::ParseError(ggen::ParseError::from_string(
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

fn convert_message(msg: ast::MessageDefinition, data: &str) -> Result<MessageDefinition, BusError> {
    let mut fields = Vec::new();
    let mut names = HashSet::new();
    let mut field_indices = HashSet::new();
    for f in msg.fields.iter() {
        let cf = convert_field(f, data)?;
        if !names.insert(cf.field_name.clone()) {
            let (start, end) = f.range();
            return Err(BusError::ParseError(ggen::ParseError::from_string(
                format!(
                    "a field named `{}` already exists in this message",
                    cf.field_name
                ),
                "",
                start,
                end,
            )));
        }

        if !field_indices.insert(cf.tag) {
            let (start, end) = f.range();
            return Err(BusError::ParseError(ggen::ParseError::from_string(
                format!(
                    "a field with tag `{}` already exists in this message",
                    cf.tag
                ),
                "",
                start,
                end,
            )));
        }

        fields.push(cf);
    }
    Ok(MessageDefinition {
        name: msg.name.as_str(data).to_owned(),
        fields,
        ast: msg.clone(),
    })
}

fn convert_service(
    service: ast::ServiceDefinition,
    data: &str,
) -> Result<ServiceDefinition, BusError> {
    let mut rpcs = Vec::new();
    let mut names = HashSet::new();
    for f in &service.fields {
        let rpc = RpcDefinition {
            name: f.name.as_str(data).to_owned(),
            argument_type: f.argument_type.as_str(data).to_owned(),
            return_type: f.return_type.as_str(data).to_owned(),
            ast: f.clone(),
        };

        if names.insert(rpc.name.clone()) {
            rpcs.push(rpc);
        } else {
            let (start, end) = f.range();
            return Err(BusError::ParseError(ggen::ParseError::from_string(
                format!("an rpc named `{}` already exists in this service", rpc.name),
                "",
                start,
                end,
            )));
        }
    }

    Ok(ServiceDefinition {
        name: service.name.as_str(data).to_owned(),
        rpcs,
        ast: service,
    })
}

enum SymbolType {
    Message,
    Enum,
    Service,
}

pub fn parse_ast(data: &str) -> Result<ast::Module, BusError> {
    let (module, _, _) = ast::Module::try_match(data, 0).map_err(|e| BusError::ParseError(e))?;
    Ok(module)
}

pub fn parse(data: &str) -> Result<Module, BusError> {
    let (module, _, _) = ast::Module::try_match(data, 0).map_err(|e| BusError::ParseError(e))?;

    let mut messages = Vec::new();
    let mut enums = Vec::new();
    let mut services = Vec::new();
    let mut types = HashMap::new();
    for d in module.definitions.into_iter() {
        match d {
            ast::Definition::Message(msg) => {
                let m = convert_message(*msg, data)?;

                if !types.contains_key(&m.name) {
                    types.insert(m.name.clone(), SymbolType::Message);
                    messages.push(m);
                } else {
                    let (start, end) = m.ast.name.range();
                    return Err(BusError::ParseError(ggen::ParseError::from_string(
                        format!("the name `{}` already exists", m.name),
                        "",
                        start,
                        end,
                    )));
                }
            }
            ast::Definition::Enum(e) => {
                let e = convert_enum(*e, data)?;
                if !types.contains_key(&e.name) {
                    types.insert(e.name.clone(), SymbolType::Enum);
                    enums.push(e);
                } else {
                    let (start, end) = e.ast.name.range();
                    return Err(BusError::ParseError(ggen::ParseError::from_string(
                        format!("the name `{}` already exists", e.name),
                        "",
                        start,
                        end,
                    )));
                }
            }
            ast::Definition::Service(s) => {
                let s = convert_service(*s, data)?;
                if !types.contains_key(&s.name) {
                    types.insert(s.name.clone(), SymbolType::Service);
                    services.push(s);
                } else {
                    let (start, end) = s.ast.name.range();
                    return Err(BusError::ParseError(ggen::ParseError::from_string(
                        format!("the name `{}` already exists", s.name),
                        "",
                        start,
                        end,
                    )));
                }
            }
        }
    }

    // Validate that all types referenced by messages are resolved
    for msg in &mut messages {
        for mut field in &mut msg.fields {
            match &field.field_type {
                FieldType::Other(s) => match types.get(s.as_str()) {
                    Some(SymbolType::Message) => field.field_type = FieldType::Message(s.clone()),
                    Some(SymbolType::Enum) => field.field_type = FieldType::Enum(s.clone()),
                    _ => {
                        let (start, end) = field.ast.type_name.range();
                        return Err(BusError::ParseError(ggen::ParseError::from_string(
                            format!("unrecognized type `{}`", &s),
                            "",
                            start,
                            end,
                        )));
                    }
                },
                _ => (),
            }
        }
    }

    // Validate that all types referenced by services are resolved
    for service in &services {
        for rpc in &service.rpcs {
            match types.get(&rpc.argument_type) {
                Some(SymbolType::Message) => (),
                Some(SymbolType::Enum) => {
                    let (start, end) = rpc.ast.argument_type.range();
                    return Err(BusError::ParseError(ggen::ParseError::from_string(
                        format!("rpc arguments must be messages, not enums",),
                        "",
                        start,
                        end,
                    )));
                }
                _ => {
                    let (start, end) = rpc.ast.argument_type.range();
                    return Err(BusError::ParseError(ggen::ParseError::from_string(
                        format!("unrecognized type `{}`", rpc.argument_type),
                        "",
                        start,
                        end,
                    )));
                }
            }

            match types.get(&rpc.return_type) {
                Some(SymbolType::Message) => (),
                Some(SymbolType::Enum) => {
                    let (start, end) = rpc.ast.return_type.range();
                    return Err(BusError::ParseError(ggen::ParseError::from_string(
                        format!("rpc return types must be messages, not enums"),
                        "",
                        start,
                        end,
                    )));
                }
                _ => {
                    let (start, end) = rpc.ast.return_type.range();
                    return Err(BusError::ParseError(ggen::ParseError::from_string(
                        format!("unrecognized type `{}`", rpc.return_type),
                        "",
                        start,
                        end,
                    )));
                }
            }
        }
    }

    Ok(Module {
        messages,
        enums,
        services,
    })
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

    #[test]
    fn test_parse_service() {
        let content = r#"
message ReadRequest {}
message ReadResponse {}
enum ZZZ {}

service MyService {
    rpc read(ReadRequest) -> ReadResponse;
}
        "#;
        let module = parse(content).unwrap();
        assert_eq!(module.services.len(), 1);
        assert_eq!(module.services[0].rpcs.len(), 1);
        assert_eq!(&module.services[0].rpcs[0].name, "read");
        assert_eq!(&module.services[0].rpcs[0].argument_type, "ReadRequest");
        assert_eq!(&module.services[0].rpcs[0].return_type, "ReadResponse");
    }
}
