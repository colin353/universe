use parser::FieldType;

const DONOTEDIT: &'static str = r#"/*
 * DO NOT EDIT THIS FILE
 *
 * It was generated by car's code generator
 *
 */

// Allow dead code, since we're generating structs/accessors
#![allow(dead_code)]

// TODO: remove this
mod test_test;
"#;

const IMPORTS: &'static str = r#"
use car::{
    DeserializeOwned, EncodedStruct, EncodedStructBuilder, RepeatedField, Serialize, RefContainer, RepeatedFieldIterator, RepeatedString
};

"#;

pub fn generate<W: std::io::Write>(
    module: &parser::Module,
    w: &mut W,
) -> Result<(), std::io::Error> {
    write!(w, "{}", DONOTEDIT)?;
    write!(w, "{}", IMPORTS)?;

    for message in &module.messages {
        generate_message(&message, w)?;
    }

    Ok(())
}

fn get_type_name(f: &parser::FieldDefinition) -> String {
    let typ = match &f.field_type {
        FieldType::Tu64 => "u64",
        FieldType::Tu32 => "u32",
        FieldType::Tu16 => "u16",
        FieldType::Tu8 => "u8",
        FieldType::Tbool => "bool",
        FieldType::Tstring => "String",
        FieldType::Tfloat => "f32",
        FieldType::Tbytes => "Vec<u8>",
        FieldType::Other(s) => s.as_str(),
    };

    let typ = if let FieldType::Other(_) = &f.field_type {
        format!("{}Owned", typ)
    } else {
        typ.to_owned()
    };

    if f.repeated {
        format!("Vec<{}>", typ)
    } else {
        typ
    }
}

fn get_return_type_name(f: &parser::FieldDefinition) -> String {
    let typ = match &f.field_type {
        FieldType::Tu64 => "u64",
        FieldType::Tu32 => "u32",
        FieldType::Tu16 => "u16",
        FieldType::Tu8 => "u8",
        FieldType::Tbool => "bool",
        FieldType::Tstring => "&str",
        FieldType::Tfloat => "f32",
        FieldType::Tbytes => "&[u8]",
        FieldType::Other(s) => s.as_str(),
    };
    if f.repeated {
        format!("RepeatedField<'a, {}>", typ)
    } else if let FieldType::Other(_) = &f.field_type {
        format!("{}", typ)
    } else {
        typ.to_owned()
    }
}

fn generate_message<W: std::io::Write>(
    msg: &parser::MessageDefinition,
    w: &mut W,
) -> Result<(), std::io::Error> {
    write!(
        w,
        r#"#[derive(Clone, Debug, Default)]
struct {name}Owned {{
"#,
        name = msg.name
    )?;
    for field in &msg.fields {
        let typ = get_type_name(&field);

        write!(
            w,
            "    {name}: {typ},\n",
            name = field.field_name,
            typ = typ
        )?;
    }
    write!(w, "}}\n")?;

    write!(
        w,
        r#"#[derive(Clone)]
enum {name}<'a> {{
    Encoded(EncodedStruct<'a>),
    DecodedOwned(Box<{name}Owned>),
    DecodedReference(&'a {name}Owned),
}}


impl<'a> Default for {name}<'a> {{
    fn default() -> Self {{
        Self::DecodedOwned(Box::new({name}Owned::default()))
    }}
}}

"#,
        name = msg.name
    )?;

    // Implement Debug for the enum type
    write!(
        w,
        r#"impl<'a> std::fmt::Debug for {name}<'a> {{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {{
        f.debug_struct("{name}")
"#,
        name = msg.name
    )?;

    for (_, field) in msg.fields.iter().enumerate() {
        write!(
            w,
            r#"         .field("{field_name}", &self.get_{field_name}())
"#,
            field_name = field.field_name
        )?;
    }

    write!(
        w,
        "         .finish()
    }}
}}

"
    )?;

    // Implement Serialize for the owned version
    write!(w, "impl Serialize for {name}Owned {{\n", name = msg.name)?;

    write!(
        w,
        r#"    fn encode<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, std::io::Error> {{
        let mut builder = EncodedStructBuilder::new(writer);
"#,
    )?;

    for (_, field) in msg.fields.iter().enumerate() {
        write!(
            w,
            "        builder.push(&self.{field_name})?;\n",
            field_name = field.field_name
        )?;
    }

    write!(
        w,
        "        builder.finish()
    }}
}}

impl DeserializeOwned for {name}Owned {{
",
        name = msg.name,
    )?;

    write!(
        w,
        r#"    fn decode_owned(bytes: &[u8]) -> Result<Self, std::io::Error> {{
        let s = EncodedStruct::new(bytes)?;
        Ok(Self {{
"#,
    )?;

    for (idx, field) in msg.fields.iter().enumerate() {
        write!(
            w,
            "            {field_name}: s.get_owned({idx}).unwrap()?,\n",
            field_name = field.field_name,
            idx = idx,
        )?;
    }

    write!(
        w,
        r#"        }})
    }}
}}
"#,
    )?;

    // Implement DeserializeOwned for the non-owned version
    write!(
        w,
        r#"impl<'a> DeserializeOwned for {name}<'a> {{
    fn decode_owned(bytes: &[u8]) -> Result<Self, std::io::Error> {{
        Ok(Self::DecodedOwned(Box::new({name}Owned::decode_owned(bytes)?)))
    }}
}}
"#,
        name = msg.name,
    )?;

    // Define repeated struct
    write!(
        w,
        r#"enum Repeated{name}<'a> {{
    Encoded(RepeatedField<'a, {name}<'a>>),
    Decoded(&'a [{name}Owned]),
}}

impl<'a> std::fmt::Debug for Repeated{name}<'a> {{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {{
        match self {{
            Self::Encoded(v) => v.fmt(f),
            Self::Decoded(v) => v.fmt(f),
        }}
    }}
}}

enum Repeated{name}Iterator<'a> {{
    Encoded(RepeatedFieldIterator<'a, {name}<'a>>),
    Decoded(std::slice::Iter<'a, {name}Owned>),
}}

impl<'a> Repeated{name}<'a> {{
    pub fn iter(&'a self) -> Repeated{name}Iterator<'a> {{
        match self {{
            Self::Encoded(r) => Repeated{name}Iterator::Encoded(r.iter()),
            Self::Decoded(s) => Repeated{name}Iterator::Decoded(s.iter()),
        }}
    }}
}}

impl<'a> Iterator for Repeated{name}Iterator<'a> {{
    type Item = {name}<'a>;
    fn next(&mut self) -> Option<Self::Item> {{
        match self {{
            Self::Encoded(it) => match it.next()? {{
                RefContainer::Owned(b) => Some(*b),
                _ => None,
            }},
            Self::Decoded(it) => Some({name}::DecodedReference(it.next()?)),
        }}
    }}
}}

"#,
        name = msg.name
    )?;

    // Implement enum version
    write!(w, "impl<'a> {name}<'a> {{\n", name = msg.name)?;

    // Implement new constructor
    write!(
        w,
        r#"    pub fn new() -> Self {{
        Self::DecodedOwned(Box::new({name}Owned {{
            ..Default::default()
        }}))
    }}
"#,
        name = msg.name
    )?;

    // Implement from_bytes constructor
    write!(
        w,
        r#"    pub fn from_bytes(bytes: &'a [u8]) -> Result<Self, std::io::Error> {{
        Ok(Self::Encoded(EncodedStruct::new(bytes)?))
    }}
"#
    )?;

    // Implement to_owned, which converts to an owned type
    write!(
        w,
        r#"    pub fn to_owned(&self) -> Result<Self, std::io::Error> {{
        match self {{
            Self::DecodedOwned(t) => Ok(Self::DecodedOwned(t.clone())),
            Self::DecodedReference(t) => Ok(Self::DecodedOwned(Box::new((*t).clone()))),
            Self::Encoded(_) => Ok(Self::DecodedOwned(Box::new(self.clone_owned()?))),
        }}
    }}

    pub fn clone_owned(&self) -> Result<{name}Owned, std::io::Error> {{
        match self {{
            Self::DecodedOwned(t) => Ok(t.as_ref().clone()),
            Self::DecodedReference(t) => Ok((*t).clone()),
            Self::Encoded(t) => Ok({name}Owned {{
"#,
        name = msg.name,
    )?;

    for (idx, field) in msg.fields.iter().enumerate() {
        write!(
            w,
            "                {field_name}: t.get_owned({idx}).unwrap()?,\n",
            field_name = field.field_name,
            idx = idx,
        )?;
    }

    write!(
        w,
        r#"            }}),
        }}
    }}
"#,
    )?;

    write!(
        w,
        r#"    pub fn encode<W: std::io::Write>(&self, writer: &mut W) -> Result<usize, std::io::Error> {{
        match self {{
            Self::DecodedOwned(t) => t.encode(writer),
            Self::DecodedReference(t) => t.encode(writer),
            Self::Encoded(t) => t.encode(writer),
        }}
    }}
"#
    )?;

    // Implement field getters
    for (idx, field) in msg.fields.iter().enumerate() {
        let owned_type = get_type_name(&field);
        let mut typ = get_return_type_name(&field);
        if field.repeated {
            if let FieldType::Other(s) = &field.field_type {
                typ = format!("Repeated{name}<'a>", name = s);
            } else if field.field_type == FieldType::Tstring {
                typ = String::from("RepeatedString<'a>");
            }
        }

        write!(
            w,
            r#"    pub fn get_{name}(&'a self) -> {field_type} {{
        match self {{
"#,
            name = field.field_name,
            field_type = typ
        )?;

        if let FieldType::Other(s) = &field.field_type {
            if field.repeated {
                write!(
                    w,
                    r#"            Self::DecodedOwned(x) => Repeated{field_type}::Decoded(x.{name}.as_slice()),
            Self::DecodedReference(x) => Repeated{field_type}::Decoded(&x.{name}.as_slice()),
            // TODO: remove unwrap
            Self::Encoded(x) => Repeated{field_type}::Encoded(RepeatedField::Encoded(x.get({idx}).unwrap().unwrap())),
"#,
                    field_type = s,
                    name = field.field_name,
                    idx = idx,
                )?;
            } else {
                write!(
                    w,
                    r#"            Self::DecodedOwned(x) => {field_type}::DecodedReference(&x.{name}),
            Self::DecodedReference(x) => {field_type}::DecodedReference(&x.{name}),
            Self::Encoded(x) => {field_type}::Encoded(x.get({idx}).unwrap().unwrap()),
"#,
                    name = field.field_name,
                    field_type = typ,
                    idx = idx,
                )?;
            }
        } else if field.repeated {
            if field.field_type == FieldType::Tstring {
                write!(
                    w,
                    r#"            Self::DecodedOwned(x) => RepeatedString::Decoded(x.{name}.as_slice()),
            Self::DecodedReference(x) => RepeatedString::Decoded(x.{name}.as_slice()),
            Self::Encoded(x) => RepeatedString::Encoded(x.get({idx}).unwrap().unwrap()),
"#,
                    name = field.field_name,
                    idx = idx,
                )?;
            } else {
                write!(
                    w,
                    r#"            Self::DecodedOwned(x) => RepeatedField::DecodedReference(x.{name}.as_slice()),
            Self::DecodedReference(x) => RepeatedField::DecodedReference(x.{name}.as_slice()),
            Self::Encoded(x) => RepeatedField::Encoded(x.get({idx}).unwrap().unwrap()),
"#,
                    name = field.field_name,
                    idx = idx,
                )?;
            }
        } else if field.field_type == FieldType::Tstring {
            write!(
                w,
                r#"            Self::DecodedOwned(x) => x.{name}.as_str(),
            Self::DecodedReference(x) => x.{name}.as_str(),
            Self::Encoded(x) => x.get({idx}).unwrap().unwrap(),
"#,
                name = field.field_name,
                idx = idx,
            )?;
        } else {
            write!(
                w,
                r#"            Self::DecodedOwned(x) => x.{name},
            Self::DecodedReference(x) => x.{name},

"#,
                name = field.field_name,
            )?;

            write!(
                w,
                r#"            Self::Encoded(x) => x.get({idx}).unwrap().unwrap(),
"#,
                idx = idx,
            )?;
        }

        write!(w, "        }}\n    }}\n")?;

        // Implement setters
        if let FieldType::Other(s) = &field.field_type {
            if field.repeated {
                write!(
                    w,
                    r#"    pub fn set_{name}(&mut self, values: Vec<{field_type}Owned>) -> Result<(), std::io::Error> {{
        match self {{
            Self::Encoded(_) | Self::DecodedReference(_) => {{
                *self = self.to_owned()?;
                self.set_{name}(values)?;
            }}
            Self::DecodedOwned(v) => {{
                v.{name} = values;
            }}
        }}
        Ok(())
    }}
"#,
                    name = field.field_name,
                    field_type = s
                )?;
            } else {
                write!(
                    w,
                    r#"    pub fn set_{name}(&mut self, value: {field_type}) -> Result<(), std::io::Error> {{
        match self {{
            Self::Encoded(_) | Self::DecodedReference(_) => {{
                *self = self.to_owned()?;
                self.set_{name}(value)?;
            }}
            Self::DecodedOwned(v) => {{
                v.{name} = value.clone_owned()?;
            }}
        }}
        Ok(())
    }}
"#,
                    name = field.field_name,
                    field_type = s
                )?;
            }
        } else {
            write!(
                w,
                r#"    pub fn set_{name}(&mut self, value: {field_type}) -> Result<(), std::io::Error> {{
        match self {{
            Self::Encoded(_) | Self::DecodedReference(_) => {{
                *self = self.to_owned()?;
                self.set_{name}(value)?;
            }}
            Self::DecodedOwned(v) => {{
                v.{name} = value;
            }}
        }}
        Ok(())
    }}
"#,
                name = field.field_name,
                field_type = owned_type
            )?;
        }

        // Implement mut_... accessors
        if field.repeated {
            write!(
                w,
                r#"    pub fn mut_{name}(&mut self) -> Result<&mut {field_type}, std::io::Error> {{
        match self {{
            Self::Encoded(_) | Self::DecodedReference(_) => {{
                *self = self.to_owned()?;
                self.mut_{name}()
            }}
            Self::DecodedOwned(v) => {{
                Ok(&mut v.{name})
            }}
        }}
    }}
"#,
                name = field.field_name,
                field_type = owned_type,
            )?;
        } else if let FieldType::Other(s) = &field.field_type {
            write!(
                w,
                r#"    pub fn mut_{name}(&mut self) -> Result<&mut {field_type}Owned, std::io::Error> {{
        match self {{
            Self::Encoded(_) | Self::DecodedReference(_) => {{
                *self = self.to_owned()?;
                self.mut_{name}()
            }}
            Self::DecodedOwned(v) => {{
                Ok(&mut v.{name})
            }}
        }}
    }}
"#,
                name = field.field_name,
                field_type = s,
            )?;
        }
    }

    write!(w, "}}\n")?;

    Ok(())
}
