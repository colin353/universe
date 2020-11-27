#[macro_use]
extern crate flags;

fn main() {
    let content = parse_flags!();

    if content.len() <= 1 {
        eprintln!("must provide at least 2 arguments!");
        std::process::exit(1);
    }

    let channel = &content[0];
    let message = content[1..].join(" ");

    let client = chat_client::ChatClient::new("localhost", 6668);
    let mut msg = chat_client::Message::new();
    msg.set_channel(channel.to_string());
    msg.set_content(message);
    msg.set_user("robot".to_string());
    client.send(msg);
}
