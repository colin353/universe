#[macro_use]
extern crate flags;
extern crate merge_lib;

use std::fs;

fn main() {
    let original = define_flag!("original", String::from(""), "The original file");
    let a = define_flag!("a", String::from(""), "Version A");
    let b = define_flag!("b", String::from(""), "Version B");

    let (merged, _) = merge_lib::merge(
        &fs::read_to_string(original.value()).expect("original: file not found"),
        &fs::read_to_string(a.value()).expect("a: file not found"),
        &fs::read_to_string(b.value()).expect("b: file not found"),
    );

    print!("{}", merged);
}
