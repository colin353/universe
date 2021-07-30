#[macro_use]
extern crate flags;

use gfile::GFile;

fn main() {
    init::init();

    let destination = define_flag!(
        "destination",
        String::from("/cns/colinmerkel-website"),
        "The destination to sync files to"
    );
    let auth_hostname = define_flag!(
        "auth_hostname",
        String::from("auth.colinmerkel.xyz"),
        "The auth service hostname"
    );
    let auth_port = define_flag!("auth_port", 8888, "The auth service port");

    let files_to_sync = parse_flags!(destination, auth_hostname, auth_port);
    println!("sync files:");

    let auth = auth_client::AuthClient::new_tls(&auth_hostname.value(), auth_port.value());
    cli::load_and_check_auth(auth);

    let destination = destination.value();

    for file in files_to_sync {
        let mut dest = GFile::create(format!("{}/{}", destination, file)).unwrap();
        let mut src = GFile::open(&file).unwrap();
        let size = std::io::copy(&mut src, &mut dest).unwrap();
        println!("copied {} ({} bytes)", file, size);
    }
}
