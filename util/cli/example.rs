fn main() {
    if cli::confirm_string("yes") {
        println!("confirmed!")
    } else {
        println!("aborted!")
    }
}
