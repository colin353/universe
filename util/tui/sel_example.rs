use sel::*;
use std::io::Read;

fn main() {
    let mut buffer = String::new();
    std::io::stdin().read_to_string(&mut buffer).unwrap();
    let choices = buffer
        .split("\n")
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();

    let choice = select(choices.clone());
    if let Some(idx) = choice {
        println!("{}", choices[idx]);
    } else {
        std::process::exit(1);
    }
}
