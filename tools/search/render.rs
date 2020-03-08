use search_proto_rust::*;

pub fn result(result: &Candidate) -> tmpl::ContentsMap {
    content!(
        "filename" => result.get_filename()
    )
}
