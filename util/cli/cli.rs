use rand::Rng;
use std::process::{Command, Stdio};

pub fn edit_string(input: &str) -> Result<String, ()> {
    let editor = match std::env::var("EDITOR") {
        Ok(x) => x,
        Err(_) => String::from("nano"),
    };
    let filename = format!("/tmp/{}", rand::thread_rng().gen::<u64>());
    std::fs::write(&filename, input).unwrap();

    let output = match Command::new(&editor)
        .arg(&filename)
        .stdout(Stdio::inherit())
        .stdin(Stdio::inherit())
        .output()
    {
        Ok(out) => out,
        Err(_) => {
            println!("unable to start editor: {}", editor);
            return Err(());
        }
    };

    if !output.status.success() {
        return Err(());
    }

    std::fs::read_to_string(&filename).map_err(|_| ())
}
