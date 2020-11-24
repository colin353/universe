use std::io::BufRead;
use std::io::Write;

#[derive(Debug)]
enum IrcError {
    UnknownCommand(String),
}

#[derive(Debug)]
enum IrcMessage<'a> {
    Nick(&'a str),
    User(&'a str),
    Ping(&'a str),
    Join(&'a str),
    PrivMsg(&'a str, &'a str),
    None,
}

impl<'a> IrcMessage<'a> {
    fn from_str(line: &'a str) -> Result<Self, IrcError> {
        let components: Vec<_> = line.split(" ").collect();
        let command = match components.iter().next() {
            Some(c) => c,
            None => return Ok(IrcMessage::None),
        };

        let message = match *command {
            "NICK" => IrcMessage::Nick(&components[1]),
            "USER" => IrcMessage::User(&components[1]),
            "PING" => IrcMessage::Ping(&components[1]),
            "JOIN" => IrcMessage::Join(&components[1]),
            "PRIVMSG" => {
                if components.len() < 3 {
                    return Err(IrcError::UnknownCommand(line.to_string()));
                }
                IrcMessage::PrivMsg(&components[1], &line.rsplit(":").next().unwrap_or(""))
            }
            _ => return Err(IrcError::UnknownCommand(line.to_string())),
        };

        Ok(message)
    }
}

pub struct IrcServer {
    stream: std::net::TcpStream,
}

impl IrcServer {
    fn handle_client(&mut self) {
        let reader = std::io::BufReader::new(self.stream.try_clone().unwrap());
        for line in reader.lines() {
            let line = line.unwrap();
            let msg = IrcMessage::from_str(&line);
            if let Err(m) = msg {
                println!("unknown command: {:?}", m);
                continue;
            } else {
            }

            match msg.unwrap() {
                IrcMessage::User(username) => {
                    self.welcome(username, "Welcome to you who are already here!");
                }
                IrcMessage::Ping(server) => {
                    self.pong(server);
                }
                IrcMessage::Join(channel) => {
                    self.announce_channel(channel);
                }
                _ => continue,
            }
        }
    }

    fn welcome(&mut self, user: &str, msg: &str) {
        self.stream
            .write(format!(":localhost 001 {} :{}\n", user, msg).as_bytes())
            .unwrap();
    }

    fn pong(&mut self, server: &str) {
        self.stream.write("PONG :localhost\n".as_bytes()).unwrap();
    }

    fn announce_channel(&mut self, channel: &str) {
        self.stream
            .write(format!(":colin!colin@colinmerkel.xyz JOIN :{}\n", channel).as_bytes())
            .unwrap();
        self.stream
            .write(format!(":localhost MODE {} +nt\n", channel).as_bytes())
            .unwrap();
        self.stream
            .write(format!(":localhost 332 colin = {} :@colin\n", channel).as_bytes())
            .unwrap();
        self.stream
            .write(format!(":localhost 353 colin = {} :@colin\n", channel).as_bytes())
            .unwrap();
        self.stream
            .write(format!(":localhost 366 colin {} :End of /NAMES list.\n", channel).as_bytes())
            .unwrap();
    }
}

fn main() {
    let listener = std::net::TcpListener::bind("127.0.0.1:6667").unwrap();

    for stream in listener.incoming() {
        let mut server = IrcServer {
            stream: stream.unwrap(),
        };
        std::thread::spawn(move || {
            server.handle_client();
        });
    }
}
