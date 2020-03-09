use search_proto_rust::*;

pub fn result(c: &Candidate) -> tmpl::ContentsMap {
    let snippet_starting_line = 1 + c.get_snippet_starting_line() as usize;

    content!(
        "filename" => c.get_filename();
        "snippet" => c.get_snippet().iter().enumerate().map(|(idx, s)| content!("line_number" => idx+snippet_starting_line, "snippet" => ws_utils::escape_htmlentities(s))).collect()
    )
}

pub fn file(f: &File) -> tmpl::ContentsMap {
    content!(
        "filename" => f.get_filename(),
        "content" => base64::encode(f.get_content())
    )
}
