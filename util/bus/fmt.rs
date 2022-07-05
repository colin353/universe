use ggen::GrammarUnit;
use parser::ast;

struct Formatter<'a, W: std::io::Write> {
    content: &'a str,
    writer: W,
    indent: usize,
}

pub fn format<W: std::io::Write>(
    ast: ast::Module,
    input: &str,
    writer: &mut W,
) -> Result<(), std::io::Error> {
    let mut f = Formatter {
        content: input,
        writer: writer,
        indent: 0,
    };

    f.format(ast)
}

impl<'a, W: std::io::Write> Formatter<'a, W> {
    fn format(&mut self, module: ast::Module) -> Result<(), std::io::Error> {
        self.format_newline_comment(module.leading_comments.as_slice(), false)?;

        for defn in &module.definitions {
            match defn {
                ast::Definition::Message(m) => self.format_message(m)?,
                ast::Definition::Enum(e) => self.format_enum(e)?,
                ast::Definition::Service(s) => self.format_service(s)?,
            }
        }

        Ok(())
    }

    fn write_indent(&mut self) -> Result<(), std::io::Error> {
        write!(self.writer, "{}", " ".repeat(self.indent * 4))
    }

    fn format_comment(
        &mut self,
        comments: &[ast::WhitespaceNewlineOrComment],
        mut directly_trailing: bool,
        mut chomp_newline: bool,
    ) -> Result<(), std::io::Error> {
        let mut bare_newline_count = 0;
        for wc in comments {
            if wc.comment.is_none() && !wc.as_str(self.content).contains("\n") {
                continue;
            }

            if let Some(c) = &wc.comment {
                chomp_newline = false;
                if directly_trailing {
                    write!(self.writer, " ")?;
                } else {
                    self.write_indent()?;
                }
                self.write_unit(c)?;
                bare_newline_count = 0;
            } else {
                bare_newline_count += 1;
            }

            if bare_newline_count <= 2 {
                if chomp_newline {
                    chomp_newline = false;
                } else {
                    write!(self.writer, "\n")?;
                }
            }

            directly_trailing = false;
        }

        Ok(())
    }

    fn format_newline_comment(
        &mut self,
        comments: &[ast::WhitespaceNewlineComment],
        mut directly_trailing: bool,
    ) -> Result<(), std::io::Error> {
        let mut bare_newline_count = 0;
        for wc in comments {
            if let Some(c) = &wc.comment {
                if directly_trailing {
                    write!(self.writer, " ")?;
                } else {
                    self.write_indent()?;
                }
                self.write_unit(c)?;
                bare_newline_count = 0;
            } else {
                bare_newline_count += 1;
            }

            if bare_newline_count <= 2 {
                write!(self.writer, "\n")?;
            }

            directly_trailing = false;
        }

        Ok(())
    }

    fn format_message(&mut self, msg: &ast::MessageDefinition) -> Result<(), std::io::Error> {
        write!(
            self.writer,
            "message {name}",
            name = msg.name.as_str(self.content)
        )?;

        self.format_comment(&msg._ws2, true, false)?;

        if msg._ws3.is_empty() && msg.fields.is_empty() && msg._ws4.is_empty() {
            write!(self.writer, " {{}}\n")?;
            return self.format_comment(&msg._ws5, false, false);
        }

        write!(self.writer, " {{\n")?;
        self.indent += 1;

        self.format_comment(msg._ws3.as_slice(), false, true)?;
        for field in &msg.fields {
            self.write_indent()?;
            write!(
                self.writer,
                "{field_name}: {repeated}{type_name} = {tag}",
                field_name = field.field_name.as_str(self.content),
                repeated = if field.repeated.is_some() {
                    "repeated "
                } else {
                    ""
                },
                type_name = field.type_name.as_str(self.content),
                tag = field.tag.value,
            )?;
            self.format_newline_comment(field._trailing_newline.inner.as_slice(), true)?;
        }
        self.format_comment(msg._ws4.as_slice(), false, false)?;

        self.indent -= 1;
        write!(self.writer, "}}\n")?;

        self.format_comment(msg._ws5.as_slice(), false, false)?;

        Ok(())
    }

    fn format_enum(&mut self, e: &ast::EnumDefinition) -> Result<(), std::io::Error> {
        write!(
            self.writer,
            "enum {name}",
            name = e.name.as_str(self.content)
        )?;

        self.format_comment(&e._ws2, true, false)?;

        if e._ws3.is_empty() && e.fields.is_empty() && e._ws4.is_empty() {
            write!(self.writer, " {{}}\n")?;
            return self.format_comment(&e._ws5, false, false);
        }

        write!(self.writer, " {{\n")?;
        self.indent += 1;

        self.format_comment(e._ws3.as_slice(), false, true)?;
        for field in &e.fields {
            self.write_indent()?;
            write!(
                self.writer,
                "{field_name} = {tag}",
                field_name = field.field_name.as_str(self.content),
                tag = field.tag.value,
            )?;
            self.format_newline_comment(field._trailing_newline.inner.as_slice(), true)?;
        }
        self.format_comment(e._ws4.as_slice(), false, false)?;

        self.indent -= 1;
        write!(self.writer, "}}\n")?;

        self.format_comment(e._ws5.as_slice(), false, false)?;

        Ok(())
    }

    fn format_service(&mut self, service: &ast::ServiceDefinition) -> Result<(), std::io::Error> {
        write!(
            self.writer,
            "service {name}",
            name = service.name.as_str(self.content)
        )?;

        self.format_comment(&service._ws2, true, false)?;

        if service._ws3.is_empty() && service.fields.is_empty() && service._ws4.is_empty() {
            write!(self.writer, " {{}}\n")?;
            return self.format_comment(&service._ws5, false, false);
        }

        write!(self.writer, " {{\n")?;
        self.indent += 1;

        self.format_comment(service._ws3.as_slice(), false, true)?;
        for rpc in &service.fields {
            self.write_indent()?;
            write!(
                self.writer,
                "rpc {name}({argtype}) -> {rettype}",
                name = rpc.name.as_str(self.content),
                argtype = rpc.argument_type.as_str(self.content),
                rettype = rpc.return_type.as_str(self.content),
            )?;
            self.format_newline_comment(rpc._trailing_newline.inner.as_slice(), true)?;
        }
        self.format_comment(service._ws4.as_slice(), false, false)?;

        self.indent -= 1;
        write!(self.writer, "}}\n")?;

        self.format_comment(service._ws5.as_slice(), false, false)?;

        Ok(())
    }

    fn write_unit<G: GrammarUnit>(&mut self, unit: &G) -> Result<(), std::io::Error> {
        let (start, end) = unit.range();
        write!(self.writer, "{}", &self.content[start..end])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_fmt {
        ($input:expr, $expected:expr,) => {
            let ast = parser::parse_ast($input).unwrap();
            let mut out = Vec::new();
            format(ast, $input, &mut out).unwrap();
            let out = String::from_utf8(out).unwrap();

            let left = out.trim();
            let right = $expected.trim();
            if left != right {
                eprintln!("Got:\n\n{}\n\nExpected:\n\n{}\n\n", left, right);
            }
            assert_eq!(left, right);
        };
    }

    #[test]
    fn test_format_leading_comment() {
        let input = "// Some crazy comment
";

        let ast = parser::parse_ast(input).unwrap();
        let mut out = Vec::new();
        format(ast, input, &mut out).unwrap();
        let out = String::from_utf8(out).unwrap();
        assert_eq!(input, &out);
    }

    #[test]
    fn test_format_message() {
        let input = "message MyMessage {}

message Zoot {
    // Comment
    id: u64 = 1
    x: repeated bool = 2

    // Trailing comment
    value: Msg = 3 // field comment
}
// Suffix
";
        assert_fmt!(input, input,);
    }

    #[test]
    fn test_format_enum() {
        let input = "enum ZZZ {}

enum Quxx {
    Default = 0
    // Leading comment
    Bar = 1

    // Trailing comment
    Baz = 2
}
// Suffix
";
        assert_fmt!(input, input,);
    }

    #[test]
    fn test_format_service() {
        let input = "service MyService {}

// Lead comment
service Chat {
    // Comment
    rpc read(ReadRequest) -> ReadResponse
}
";
        assert_fmt!(input, input,);
    }
}
