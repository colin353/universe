#[macro_use]
extern crate lazy_static;

use search_proto_rust::*;

mod default;
mod rust;

pub fn get_filetype(filename: &str) -> FileType {
    if filename.ends_with(".rs") {
        return FileType::RUST;
    }
    FileType::UNKNOWN
}

pub fn extract_keywords(file: &File) -> Vec<ExtractedKeyword> {
    match get_filetype(file.get_filename()) {
        FileType::RUST => rust::extract_keywords(file),
        _ => default::extract_keywords(file),
    }
}
