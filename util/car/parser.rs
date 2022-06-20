mod ast;

pub struct ParsedModule {
    messages: Vec<MessageDefinition>,
}

pub struct MessageDefinition {
    fields: Vec<FieldDefinition>,
}

pub struct FieldDefinition {
    repeated: bool,
    type_name: String,
    field_name: String,
    tag_number: u32,
}

pub fn parse(data: &str) -> Result<ParsedModule, ggen::ParseError> {
    Ok(ParsedModule {
        messages: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
}
