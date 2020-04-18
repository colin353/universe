#[macro_use]
extern crate flags;

fn main() {
    let auth_hostname = define_flag!(
        "auth_hostname",
        String::from("auth.colinmerkel.xyz"),
        "the hostname for auth service"
    );
    let auth_port = define_flag!("auth_port", 8888, "the port for auth service");
    parse_flags!(auth_hostname, auth_port);

    let auth = auth_client::AuthClient::new(&auth_hostname.value(), auth_port.value());
    cli::load_and_check_auth(auth);

    eprintln!("✔️ Authentication succeeded");
}
