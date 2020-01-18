extern crate json;
extern crate rand;

#[macro_use]
extern crate flags;
extern crate recordio;
extern crate x20_client;
extern crate x20_grpc_rust as x20;

mod config;
mod subprocess;
mod util;

use std::io::Read;

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
    let env = define_flag!("env", String::new(), "The environment to use");

    let args = parse_flags!(name, path, target, create, x20_hostname, x20_port, env);
    if args.len() != 1 {
        return usage();
    }

    let client = x20_client::X20Client::new(&x20_hostname.value(), x20_port.value());
    let base_dir = format!("{}/.x20", std::env::home_dir().unwrap().to_str().unwrap());
    let manager = util::X20Manager::new(client, base_dir);

    match args[0].as_ref() {
        "list" => {
            manager.list();
        }
        "ls" => {
            manager.list();
        }
        "publish" => {
            manager.publish(name.value(), path.value(), target.value(), create.value());
        }
        "update" => {
            manager.update();
        }
        "env" => {
            manager.write_saved_environment(env.value());
            println!("✔️ Updated environment to `{}`", env.value());
            manager.update();
        }
        "setconfig" => {
            let mut buffer = String::new();
            std::io::stdin().read_to_string(&mut buffer).unwrap();
            manager.setconfig(buffer);
        }
        "start" => manager.start(),
        x => {
            println!("Unknown command: {}", x);
            std::process::exit(1);
        }
    }
}
