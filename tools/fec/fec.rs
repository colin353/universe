#[macro_use]
extern crate flags;

use std::collections::HashSet;
use std::io::Write;

fn main() {
    let output = define_flag!("output", String::new(), "location of the compiled output");
    let prefix = define_flag!(
        "prefix",
        String::new(),
        "a prefix to strip when writing outputs"
    );
    let inputs = parse_flags!(output, prefix);

    if inputs.len() == 0 {
        eprintln!("must specify an input file");
        std::process::exit(1);
    }
    println!("output = {}", output.value());

    // If necessary, create the parent directories
    let path_string = output.value();
    let mut path = std::path::PathBuf::from(&path_string);

    if path_string.ends_with(".js") | path_string.ends_with(".mjs") {
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p);
        }
    }

    let mut input_js = HashSet::new();
    for input in inputs {
        if input.ends_with(".js") || input.ends_with(".mjs") {
            input_js.insert(input);
        } else if input.ends_with(".html") {
            let mut js = input[..input.len() - 5].to_string();
            js += ".mjs";
            input_js.insert(js);
        } else if input.ends_with(".css") {
            let mut js = input[..input.len() - 4].to_string();
            js += ".mjs";
            input_js.insert(js);
        }
    }

    if input_js.len() == 0 {
        eprintln!("must specify at least one javascript input (*.js or *.mjs)");
        std::process::exit(1);
    }

    let prefix = prefix.value();

    for input in input_js.iter() {
        let input_path = std::path::Path::new(&input);

        // Strip the prefix off and create parent directories
        let mut output_path = if path_string.ends_with(".js") || path_string.ends_with(".mjs") {
            std::path::PathBuf::from(&path_string)
        } else {
            let mut p = path.join(
                input_path
                    .strip_prefix(&prefix)
                    .unwrap()
                    .parent()
                    .unwrap()
                    .to_owned(),
            );

            std::fs::create_dir_all(&p).unwrap();
            p.push(input_path.file_name().unwrap());
            p
        };

        let mut f = std::fs::File::create(&output_path).unwrap();
        let mut compiler = fec_lib::FECompiler::new();
        compiler.compile(input);
        if compiler.success() {
            f.write(&compiler.result.as_bytes()).unwrap();
        } else {
            for error in &compiler.errors {
                eprintln!("err: {:?}", error);
            }
        }
        println!("compiled {:?}", output_path);
    }
}
