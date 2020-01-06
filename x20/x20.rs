extern crate rand;

#[macro_use]
extern crate flags;
extern crate x20_client;
extern crate x20_grpc_rust as x20;

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
    let x20_hostname = define_flag!(
        "x20_hostname",
        String::from("x20.colinmerkel.xyz"),
        "The hostname of the x20 service"
    );
    let x20_port = define_flag!("x20_port", 8009, "The port of the x20 service");

    let args = parse_flags!(name, path, target, create, x20_hostname, x20_port);
    if args.len() != 1 {
        return usage();
    }

    let client = x20_client::X20Client::new(&x20_hostname.value(), x20_port.value());
    let manager = util::X20Manager::new(client);

    match args[0].as_ref() {
        "list" => {
            manager.list();
        }
        "publish" => {
            manager.publish(name.value(), path.value(), target.value(), create.value());
        }
        x => {
            println!("Unknown command: {}", x);
            std::process::exit(1);
        }
    }
}
