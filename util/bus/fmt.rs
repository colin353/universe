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
        self.format_newline_comment(module.leading_comments.as_slice(), false, true, false)?;

        for (idx, defn) in module.definitions.iter().enumerate() {
            if idx != 0 {
                write!(self.writer, "\n\n")?;
            }
            match defn {
                ast::Definition::Message(m) => self.format_message(m)?,
                ast::Definition::Enum(e) => self.format_enum(e)?,
                ast::Definition::Service(s) => self.format_service(s)?,
            }
        }

        if self.contains_newline_comments(&module.trailing_comments) {
            write!(self.writer, "\n")?;
        }
        self.format_newline_comment(module.trailing_comments.as_slice(), false, false, true)?;

        Ok(())
    }

    fn write_indent(&mut self) -> Result<(), std::io::Error> {
        write!(self.writer, "{}", " ".repeat(self.indent * 4))
    }

    fn format_comment(
        &mut self,
        comments: &[ast::WhitespaceNewlineOrComment],
        mut directly_trailing: bool,
        mut drop_leading_newlines: bool,
        drop_trailing_newlines: bool,
    ) -> Result<(), std::io::Error> {
        let mut accumulated_newlines = 0;
        for wc in comments {
            if wc.comment.is_none() && !wc.as_str(self.content).contains("\n") {
                continue;
            }

            if let Some(c) = &wc.comment {
                if drop_leading_newlines {
                    accumulated_newlines = 0;
                    drop_leading_newlines = false;
                }

                if accumulated_newlines > 0 {
                    write!(
                        self.writer,
                        "{}",
                        "\n".repeat(std::cmp::min(2, accumulated_newlines))
                    )?;
                    accumulated_newlines = 0;
                }

                if directly_trailing {
                    write!(self.writer, " ")?;
                } else {
                    self.write_indent()?;
                }

                let (start, end) = c.range();
                let data = &self.content[start..end];

                let trimmed = if drop_leading_newlines {
                    data.trim_start()
                } else {
                    data
                };

                let end_trimmed = trimmed.trim_end();
                accumulated_newlines += trimmed[end_trimmed.len()..].matches('\n').count();
                write!(self.writer, "{}", end_trimmed)?;
            }

            accumulated_newlines += 1;

            directly_trailing = false;
        }

        if drop_leading_newlines {
            // Drop newlines
        } else if accumulated_newlines > 0 && !drop_trailing_newlines {
            write!(
                self.writer,
                "{}",
                "\n".repeat(std::cmp::min(2, accumulated_newlines))
            )?;
        }

        Ok(())
    }

    fn format_newline_comment(
        &mut self,
        comments: &[ast::WhitespaceNewlineComment],
        mut directly_trailing: bool,
        mut drop_leading_newlines: bool,
        drop_trailing_newlines: bool,
    ) -> Result<(), std::io::Error> {
        let mut accumulated_newlines = 0;
        for wc in comments {
            if wc.comment.is_none() && !wc.as_str(self.content).contains("\n") {
                continue;
            }

            if let Some(c) = &wc.comment {
                if drop_leading_newlines {
                    accumulated_newlines = 0;
                    drop_leading_newlines = false;
                }

                if accumulated_newlines > 0 {
                    write!(
                        self.writer,
                        "{}",
                        "\n".repeat(std::cmp::min(2, accumulated_newlines))
                    )?;
                    accumulated_newlines = 0;
                }

                if directly_trailing {
                    write!(self.writer, " ")?;
                } else {
                    self.write_indent()?;
                }

                let (start, end) = c.range();
                let data = &self.content[start..end];

                let trimmed = if drop_leading_newlines {
                    data.trim_start()
                } else {
                    data
                };

                let end_trimmed = trimmed.trim_end();
                accumulated_newlines += trimmed[end_trimmed.len()..].matches('\n').count();
                write!(self.writer, "{}", end_trimmed)?;
            }

            accumulated_newlines += 1;

            directly_trailing = false;
        }

        if drop_leading_newlines {
            // Drop newlines
        } else if accumulated_newlines > 0 && !drop_trailing_newlines {
            write!(
                self.writer,
                "{}",
                "\n".repeat(std::cmp::min(2, accumulated_newlines))
            )?;
        }

        Ok(())
    }

    fn format_message(&mut self, msg: &ast::MessageDefinition) -> Result<(), std::io::Error> {
        self.format_newline_comment(&msg._leading_comments, false, false, true)?;

        write!(
            self.writer,
            "message {name}",
            name = msg.name.as_str(self.content)
        )?;

        self.format_comment(&msg._ws2, true, false, true)?;

        if !self.contains_comments(&msg._ws3)
            && msg.fields.is_empty()
            && !self.contains_comments(&msg._ws4)
        {
            write!(self.writer, " {{}}")?;
            if let Some(c) = &msg._ws5 {
                self.format_newline_comment(&[c.clone()], false, false, true)?;
            }
            return Ok(());
        }

        write!(self.writer, " {{\n")?;
        self.indent += 1;

        self.format_comment(&msg._ws3, false, true, false)?;
        for (idx, field) in msg.fields.iter().enumerate() {
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
            let is_last_field = idx == msg.fields.len() - 1;
            self.format_newline_comment(
                field._trailing_newline.inner.as_slice(),
                true,
                false,
                is_last_field,
            )?;
        }
        self.format_comment(msg._ws4.as_slice(), false, false, true)?;

        self.indent -= 1;
        write!(self.writer, "\n}}")?;

        if let Some(c) = &msg._ws5 {
            self.format_newline_comment(&[c.clone()], false, false, true)?;
        }

        Ok(())
    }

    fn contains_comments(&mut self, c: &[ast::WhitespaceNewlineOrComment]) -> bool {
        c.iter().any(|x| x.comment.is_some())
    }

    fn contains_newline_comments(&mut self, c: &[ast::WhitespaceNewlineComment]) -> bool {
        c.iter().any(|x| x.comment.is_some())
    }

    fn format_enum(&mut self, e: &ast::EnumDefinition) -> Result<(), std::io::Error> {
        self.format_newline_comment(&e._leading_comments, false, true, false)?;

        write!(
            self.writer,
            "enum {name}",
            name = e.name.as_str(self.content)
        )?;

        self.format_comment(&e._ws2, true, false, true)?;

        if !self.contains_comments(&e._ws3)
            && e.fields.is_empty()
            && !self.contains_comments(&e._ws4)
        {
            write!(self.writer, " {{}}")?;
            if let Some(c) = &e._ws5 {
                self.format_newline_comment(&[c.clone()], false, false, true)?;
            }
            return Ok(());
        }

        write!(self.writer, " {{\n")?;
        self.indent += 1;

        self.format_comment(e._ws3.as_slice(), false, true, false)?;
        if self.contains_comments(&e._ws3) {
            write!(self.writer, "\n")?;
        }

        for (idx, field) in e.fields.iter().enumerate() {
            self.write_indent()?;
            write!(
                self.writer,
                "{field_name} = {tag}",
                field_name = field.field_name.as_str(self.content),
                tag = field.tag.value,
            )?;
            let is_last_field = idx == e.fields.len() - 1;
            self.format_newline_comment(
                field._trailing_newline.inner.as_slice(),
                true,
                false,
                is_last_field,
            )?;
        }
        self.format_comment(e._ws4.as_slice(), false, false, true)?;

        self.indent -= 1;
        write!(self.writer, "\n}}")?;

        if let Some(c) = &e._ws5 {
            self.format_newline_comment(&[c.clone()], false, false, true)?;
        }

        Ok(())
    }

    fn format_service(&mut self, service: &ast::ServiceDefinition) -> Result<(), std::io::Error> {
        self.format_newline_comment(&service._leading_comments, false, true, false)?;

        write!(
            self.writer,
            "service {name}",
            name = service.name.as_str(self.content)
        )?;

        self.format_comment(&service._ws2, true, false, true)?;

        if !self.contains_comments(&service._ws3)
            && service.fields.is_empty()
            && !self.contains_comments(&service._ws4)
        {
            write!(self.writer, " {{}}")?;
            if let Some(c) = &service._ws5 {
                self.format_newline_comment(&[c.clone()], false, false, true)?;
            }
            return Ok(());
        }

        write!(self.writer, " {{\n")?;
        self.indent += 1;

        self.format_comment(&service._ws3, false, true, false)?;
        for (idx, rpc) in service.fields.iter().enumerate() {
            self.write_indent()?;
            write!(
                self.writer,
                "rpc {name}({argtype}) -> {rettype}",
                name = rpc.name.as_str(self.content),
                argtype = rpc.argument_type.as_str(self.content),
                rettype = rpc.return_type.as_str(self.content),
            )?;

            let is_last_field = idx == service.fields.len() - 1;
            self.format_newline_comment(
                rpc._trailing_newline.inner.as_slice(),
                true,
                false,
                is_last_field,
            )?;
        }
        self.format_comment(service._ws4.as_slice(), false, false, true)?;

        self.indent -= 1;
        write!(self.writer, "\n}}")?;

        if let Some(c) = &service._ws5 {
            self.format_newline_comment(&[c.clone()], false, false, true)?;
        }

        Ok(())
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
    fn test_format_simple_enum() {
        let input = "enum Zoot {
    Default = 0
}
";
        assert_fmt!(input, input,);
    }

    #[test]
    fn test_format_simple_msg() {
        let input = "message Zoot {
    id: u64 = 1
}
";
        assert_fmt!(input, input,);
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

    // Another comment
    rpc write(WriteRequest) -> WriteResponse
}
";
        assert_fmt!(input, input,);
    }

    #[test]
    fn test_format_newlines() {
        let input = "service MyService {
}



";
        let expected = "service MyService {}
";

        assert_fmt!(input, expected,);
    }

    #[test]
    fn test_format_intermediate_newlines() {
        let input = "
service MyService {}
service AnotherOne {}


service Third {}
";
        let expected = "
service MyService {}

service AnotherOne {}

service Third {}
";

        assert_fmt!(input, expected,);
    }

    #[test]
    fn test_comment_newline() {
        let input = "
message XYZ {
    id: u64 = 1;

    // Pre comment


    num: u64 = 2;

}
";
        let expected = "
message XYZ {
    id: u64 = 1

    // Pre comment

    num: u64 = 2
}
";

        assert_fmt!(input, expected,);
    }

    #[test]
    fn test_enforced_space() {
        let input = "
message XYZ {
    id: u64 = 1;
}
message ABC {
    size: u32 = 1;
}
";
        let expected = "
message XYZ {
    id: u64 = 1
}

message ABC {
    size: u32 = 1
}
";

        assert_fmt!(input, expected,);
    }
}
