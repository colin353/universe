use search_proto_rust::*;

pub fn result(c: &Candidate) -> tmpl::ContentsMap {
    content!(
        "filename" => c.get_filename()
    )
}

pub fn file(f: &File) -> tmpl::ContentsMap {
    content!(
        "filename" => f.get_filename(),
        "content" => base64::encode(f.get_content())
    )
}
