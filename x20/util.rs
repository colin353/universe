pub fn publish(name: String, path: String, target: String, create: bool) {
    if name.is_empty() && target.is_empty() {
        eprintln!("You must specify either a name (--name) or a target (--target) to publish");
        std::process::exit(1);
    }

    println!("publish artifact");
}
