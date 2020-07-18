#[macro_use]
extern crate flags;

use std::io::Write;

fn main() {
    let output = define_flag!("output", String::new(), "location of the compiled output");
    let input = parse_flags!(output);

    if input.len() != 1 {
        eprintln!("must specify exactly one input file");
        std::process::exit(1);
    }

    // If necessary, create the parent directories
    let path_string = output.value();
    let path = std::path::Path::new(&path_string);
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p).unwrap();
    }

    let mut f = std::fs::File::create(path).unwrap();
    let mut compiler = fec_lib::FECompiler::new();
    compiler.compile(&input[0]);
    if compiler.success() {
        f.write(&compiler.result.as_bytes()).unwrap();
    } else {
        for error in &compiler.errors {
            eprintln!("err: {:?}", error);
        }
    }
}
