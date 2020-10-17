extern crate init;
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
    let source = define_flag!(
        "source",
        String::from(""),
        "The source path (i.e. WRT the root of the repo) of the file you are publishing"
    );
    let docker_img = define_flag!(
        "docker_img",
        String::from(""),
        "The name of the docker image you are publishing"
    );
    let docker_img_tag = define_flag!(
        "docker_img_tag",
        String::from(""),
        "The tag of the docker image you are publishing"
    );
    let create = define_flag!("create", false, "Whether or not to create a new binary");
    let x20_hostname = define_flag!(
        "x20_hostname",
        String::from("x20.colinmerkel.xyz"),
        "The hostname of the x20 service"
    );
    let x20_port = define_flag!("x20_port", 8010, "The port of the x20 service");
    let env = define_flag!("env", String::new(), "The environment to use");

    let args = parse_flags!(
        name,
        path,
        target,
        create,
        source,
        x20_hostname,
        x20_port,
        env,
        docker_img,
        docker_img_tag
    );
    if args.len() != 1 {
        return usage();
    }

    init::init();

    // We may not have valid authentication on initial bootstrap, and that's OK. Just proceed
    // without errors, since in that scenario we won't be doing auth-required actions like
    // publishing binaries.
    let token = cli::load_auth();
    let client = x20_client::X20Client::new_tls(&x20_hostname.value(), x20_port.value(), token);
    let base_dir = format!("{}/.x20", std::env::home_dir().unwrap().to_str().unwrap());
    let manager = util::X20Manager::new(client, base_dir);

    match args[0].as_ref() {
        "list" | "ls" => {
            manager.list();
        }
        "publish" => {
            manager.publish(
                name.value(),
                path.path(),
                target.value(),
                source.value(),
                docker_img.value(),
                docker_img_tag.value(),
                create.value(),
            );
        }
        "delete_binary" => manager.delete_binary(name.value()),
        "delete_config" => {
            let mut buffer = String::new();
            std::io::stdin().read_to_string(&mut buffer).unwrap();
            manager.delete_config(buffer);
        }
        "build" => manager.build(name.value()),
        "update" => {
            manager.update();
        }
        "setenv" | "env" => {
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
