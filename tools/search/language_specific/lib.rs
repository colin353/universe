#[macro_use]
extern crate lazy_static;

use search_proto_rust::*;

mod default;
mod javascript;
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
    if filename.ends_with(".js") || filename.ends_with(".ts") || filename.ends_with(".mjs") {
        return FileType::JAVASCRIPT;
    }
    if filename.ends_with("BUILD") || filename == "WORKSPACE" {
        return FileType::BAZEL;
    }
    if filename.ends_with(".py") {
        return FileType::PYTHON;
    }
    if filename.ends_with(".h") || filename.ends_with(".c") {
        return FileType::C;
    }
    if filename.ends_with(".hpp") || filename.ends_with(".cpp") {
        return FileType::CPP;
    }

    FileType::UNKNOWN
}

pub fn annotate_file(file: &mut File) {
    match get_filetype(file.get_filename()) {
        FileType::RUST => rust::annotate_file(file),
        FileType::PYTHON => python::annotate_file(file),
        FileType::JAVASCRIPT => javascript::annotate_file(file),
        _ => (),
    }
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
        FileType::JAVASCRIPT => javascript::extract_definitions(file),
        _ => default::extract_definitions(file),
    }
}
