use search_grpc_rust::*;

pub fn result(c: &Candidate) -> tmpl::ContentsMap {
    let snippet_starting_line = 1 + c.get_snippet_starting_line() as usize;

    content!(
        "filename" => c.get_filename(),
        "is_directory" => c.get_is_directory(),
        "jump_to_line" => c.get_jump_to_line() + 1;
        "snippet" => c.get_snippet().iter().enumerate().map(|(idx, s)| content!("line_number" => idx+snippet_starting_line, "snippet" => ws_utils::escape_htmlentities(s))).collect(),
        "child_directories" => c.get_child_directories().iter().map(|s| content!("child" => s)).collect(),
        "child_files" => c.get_child_files().iter().map(|s| content!("child" => s)).collect()
    )
}

pub fn file(f: &File) -> tmpl::ContentsMap {
    content!(
         "filename" => f.get_filename(),
         "content" => base64::encode(f.get_content());
         "child_directories" => f.get_child_directories().iter().map(|s| content!("child" => s)).collect(),
         "child_files" => f.get_child_files().iter().map(|s| content!("child" => s)).collect()
    )
}
