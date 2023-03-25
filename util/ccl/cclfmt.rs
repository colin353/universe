#[macro_use]
extern crate flags;

use std::io::Read;

fn main() {
    let overwrite = define_flag!("overwrite", false, "whether to overwrite the provided file");
    let args = parse_flags!(overwrite);

    let content = match args.iter().next() {
        Some(f) => std::fs::read_to_string(f).unwrap(),
        None => {
            // Read from stdin
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf);

            // Add a newline to the end if it's missing
            if !buf.ends_with('\n') {
                buf.push('\n');
            }
            buf
        }
    };

    let parsed = ccl::get_ast_or_panic(&content);
    println!("{}", ccl::format(parsed, &content));
}
