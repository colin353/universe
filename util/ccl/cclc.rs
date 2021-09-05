#[macro_use]
extern crate flags;

use std::io::Read;

fn main() {
    let args = parse_flags!();
    let mut args = args.into_iter();
    let content = match args.next() {
        Some(f) => std::fs::read_to_string(f).unwrap(),
        None => {
            // Read from stdin
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf);
            buf
        }
    };

    let specifier = args.next().unwrap_or(String::new());

    let parsed = ccl::get_ast_or_panic(&content);
    let resolved = match ccl::exec(parsed, &content, &specifier) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("failed to evaluate!\n\n{}", e.render(&content));
            std::process::exit(1);
        }
    };

    println!("{:?}", resolved);
}
