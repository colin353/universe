use search_proto_rust::*;

mod default;

pub fn get_filetype(filename: &str) -> FileType {
    FileType::UNKNOWN
}

pub fn extract_keywords(file: &File) -> Vec<ExtractedKeyword> {
    match get_filetype(file.get_filename()) {
        _ => default::extract_keywords(file),
    }
}
