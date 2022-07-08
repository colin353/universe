use std::io::Read;

mod rust;

fn main() {
    let language = flags::define_flag!(
        "language",
        String::from("rust"),
        "The language to generate code for"
    );

    let args = flags::parse_flags!(language);
    let content = if args.len() == 1 {
        match std::fs::read_to_string(&args[0]) {
            Ok(s) => s,
            Err(_) => {
                eprintln!("unable to read file `{}`", &args[0]);
                std::process::exit(1);
            }
        }
    } else {
        let stdin = std::io::stdin();
        let mut handle = stdin.lock();
        let mut buf = String::new();
        handle.read_to_string(&mut buf).unwrap();
        buf
    };

    let module = match parser::parse(&content) {
        Ok(m) => m,
        Err(e) => {
            // Nicely render the error
            match e {
                parser::BusError::ParseError(e) => {
                    eprintln!("{}", e.render(&content));
                }
            }
            std::process::exit(1);
        }
    };

    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    match language.value().as_str() {
        "rust" => rust::generate(&module, &mut out).unwrap(),
        lang => {
            eprintln!("unrecognized language `{}`", lang);
            std::process::exit(1);
        }
    }
}
