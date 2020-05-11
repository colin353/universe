use search_proto_rust::*;

pub fn annotate_file(file: &mut File) {
    if file.get_filename().ends_with(".jest.js")
        || file.get_filename().ends_with(".test.js")
        || file.get_filename().ends_with("/__snapshots__")
    {
        file.set_is_test(true);
    }
}
