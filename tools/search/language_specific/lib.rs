#[macro_use]
extern crate lazy_static;

use search_proto_rust::*;

mod default;
mod proto;
mod python;
mod rust;

pub fn get_filetype(filename: &str) -> FileType {
    if filename.ends_with(".rs") {
        return FileType::RUST;
    }
    if filename.ends_with(".html") || filename.ends_with(".htm") {
        return FileType::HTML;
    }
    if filename.ends_with(".proto") {
        return FileType::PROTO;
    }
    if filename.ends_with(".js") {
        return FileType::JAVASCRIPT;
    }
    if filename.ends_with("BUILD") || filename == "WORKSPACE" {
        return FileType::BAZEL;
    }
    if filename.ends_with(".py") {
        return FileType::PYTHON;
    }
    FileType::UNKNOWN
}

pub fn extract_keywords(file: &File) -> Vec<ExtractedKeyword> {
    match get_filetype(file.get_filename()) {
        FileType::RUST => rust::extract_keywords(file),
        FileType::PROTO => proto::extract_keywords(file),
        FileType::PYTHON => python::extract_keywords(file),
        _ => default::extract_keywords(file),
    }
}
pub fn extract_definitions(file: &File) -> Vec<SymbolDefinition> {
    match get_filetype(file.get_filename()) {
        FileType::RUST => rust::extract_definitions(file),
        FileType::PROTO => proto::extract_definitions(file),
        FileType::PYTHON => python::extract_definitions(file),
        _ => default::extract_definitions(file),
    }
}
