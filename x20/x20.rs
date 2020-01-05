#[macro_use]
extern crate flags;

mod util;

fn usage() {
    println!("USAGE: x20 <command>");
    println!("use x20 --help for details.");
}

fn main() {
    let name = define_flag!(
        "name",
        String::from(""),
        "The name of the binary you are publishing"
    );
    let path = define_flag!(
        "path",
        String::from(""),
        "The path to the binary you are publishing"
    );
    let target = define_flag!("target", String::from(""), "The target you are publishing");
    let create = define_flag!("create", false, "Whether or not to create a new binary");

    let args = parse_flags!(name, path, target, create);
    if args.len() != 1 {
        return usage();
    }

    match args[0].as_ref() {
        "publish" => {
            util::publish(name.value(), path.value(), target.value(), create.value());
        }
        x => println!("Unknown command: {}", x),
    }
}
