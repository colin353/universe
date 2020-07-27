#[macro_use]
extern crate flags;

use std::collections::HashSet;
use std::io::Write;

fn main() {
    let output = define_flag!("output", String::new(), "location of the compiled output");
    let inputs = parse_flags!(output);

    if inputs.len() == 0 {
        eprintln!("must specify an input file");
        std::process::exit(1);
    }

    // If necessary, create the parent directories
    let path_string = output.value();
    let mut path = std::path::PathBuf::from(&path_string);

    if path_string.ends_with(".js") {
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p);
        }
    } else {
        let last_dir = path.file_name().unwrap().to_owned();
        path.push(last_dir);
        std::fs::create_dir_all(&path);
    }

    let mut input_js = HashSet::new();
    for input in inputs {
        if input.ends_with(".js") {
            input_js.insert(input);
        } else if input.ends_with(".html") {
            let mut js = input[..input.len() - 5].to_string();
            js += ".js";
            input_js.insert(js);
        } else if input.ends_with(".css") {
            let mut js = input[..input.len() - 4].to_string();
            js += ".js";
            input_js.insert(js);
        }
    }

    if input_js.len() == 0 {
        eprintln!("must specify at least one javascript input (*.js)");
        std::process::exit(1);
    }

    for input in input_js.iter() {
        let input_path = std::path::Path::new(&input);

        if !path_string.ends_with(".js") {
            path.set_file_name(input_path.file_name().unwrap());
        }

        let mut f = std::fs::File::create(&path).unwrap();
        let mut compiler = fec_lib::FECompiler::new();
        compiler.compile(input);
        if compiler.success() {
            f.write(&compiler.result.as_bytes()).unwrap();
        } else {
            for error in &compiler.errors {
                eprintln!("err: {:?}", error);
            }
        }
        println!("compiled {:?}", path);
    }
}
