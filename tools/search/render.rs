use search_grpc_rust::*;

pub fn result(c: &Candidate) -> tmpl::ContentsMap {
    let snippet_starting_line = c.get_snippet_starting_line() as usize;
    let code = c
        .get_snippet()
        .iter()
        .map(|x| x.to_string())
        .collect::<Vec<_>>()
        .join("\n");

    content!(
        "filename" => c.get_filename(),
        "is_directory" => c.get_is_directory(),
        "code" => base64::encode(&code),
        "language" => format!("{:?}", c.get_file_type()).to_lowercase(),
        "snippet_starting_line" => snippet_starting_line,
        "jump_to_line" => c.get_jump_to_line() + 1;
        "child_directories" => c.get_child_directories().iter().map(|s| content!("child" => s)).collect(),
        "child_files" => c.get_child_files().iter().map(|s| content!("child" => s)).collect()
    )
}

pub fn symbol(s: &SymbolDefinition) -> tmpl::ContentsMap {
    content!(
        "type" => format!("{:?}", s.get_symbol_type()),
        "symbol" => s.get_symbol(),
        "start" => s.get_line_number(),
        "end" => s.get_end_line_number()
    )
}

pub fn file(f: &File, content: &str) -> tmpl::ContentsMap {
    content!(
         "filename" => f.get_filename(),
         "type" => format!("{:?}", f.get_file_type()).to_lowercase(),
         "content" => base64::encode(content);
         "child_directories" => f.get_child_directories().iter().map(|s| content!("child" => s)).collect(),
         "child_files" => f.get_child_files().iter().map(|s| content!("child" => s)).collect(),
         "symbols" => f.get_symbols().iter().filter(|s| s.get_end_line_number() != 0 ).map(|s| symbol(s)).collect()
    )
}

pub fn entity(e: &EntityInfo) -> tmpl::ContentsMap {
    content!(
        "name" => e.get_name(),
        "kind" => entity_kind(e.get_kind()),
        "filename" => e.get_file(),
        "line_number" => e.get_line_number() + 1,
        "language" => entity_language(e.get_file_type());
        "subinfos" => e.get_subinfos().iter().map(|s| subinfo(s)).collect()
    )
}

pub fn subinfo(e: &EntitySubInfo) -> tmpl::ContentsMap {
    content!(
        "name" => e.get_name();
        "infos" => e.get_item_texts().iter().zip(e.get_links().iter()).take(5).map(|(text, link)| {
            content!(
                "text" => text,
                "link" => link
            )
        }).collect()
    )
}

fn entity_kind(e: EntityKind) -> &'static str {
    match e {
        EntityKind::E_UNKNOWN => "",
        EntityKind::E_TARGET => "target",
        EntityKind::E_FUNCTION => "function",
        EntityKind::E_STRUCT => "structure",
        EntityKind::E_PROJECT => "project",
        EntityKind::E_TRAIT => "trait",
    }
}

fn entity_language(f: FileType) -> String {
    match f {
        FileType::UNKNOWN => String::new(),
        x => format!("{:?}", x).to_lowercase(),
    }
}

pub fn entity_info(e: &EntityInfo) -> json::JsonValue {
    let mut obj = json::object::Object::new();
    obj["name"] = e.get_name().into();
    obj["kind"] = entity_kind(e.get_kind()).into();
    obj["file_type"] = entity_language(e.get_file_type()).into();

    obj["file"] = e.get_file().into();
    obj["line_number"] = e.get_line_number().into();
    obj.into()
}
