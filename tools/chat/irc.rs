use chat_grpc_rust::Message;
use chat_service::{ChatServiceEvent, ChatServiceHandler};
use futures::future::Future;
use futures::stream::Stream;
use futures::StreamExt;
use std::io::BufRead;
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;

#[derive(Debug)]
enum IrcError {
    UnknownCommand(String),
}

#[derive(Debug)]
enum IrcMessage<'a> {
    Nick(&'a str),
    User(&'a str),
    Mode(&'a str),
    Ping(&'a str),
    Join(&'a str),
    PrivMsg(&'a str, &'a str),
    None,
}

impl<'a> IrcMessage<'a> {
    fn from_str(line: &'a str) -> Result<Self, IrcError> {
        println!("from_str: {}", line);

        let components: Vec<_> = line.split(" ").collect();
        let command = match components.iter().next() {
            Some(c) => c,
            None => return Ok(IrcMessage::None),
        };

        let message = match *command {
            "NICK" => IrcMessage::Nick(&components[1]),
            "USER" => IrcMessage::User(&components[1]),
            "PING" => IrcMessage::Ping(&components[1]),
            "MODE" => IrcMessage::Mode(&components[1]),
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

#[derive(Clone)]
pub struct IrcServer {
    chat: ChatServiceHandler,
}

impl IrcServer {
    pub fn new(chat: ChatServiceHandler) -> Self {
        Self { chat }
    }

    pub fn handle_pre_auth(
        &self,
        lines_iter: &mut std::io::Lines<std::io::BufReader<std::net::TcpStream>>,
    ) -> Option<String> {
        while let Some(line) = lines_iter.next() {
            let line = line.unwrap();

            let msg = IrcMessage::from_str(&line);
            if let Err(m) = msg {
                println!("unknown command: {:?}", m);
                continue;
            }

            match msg.unwrap() {
                IrcMessage::User(username) => {
                    return Some(username.to_string());
                }
                m => {
                    println!("got message: {:?}", m);
                }
            }
        }

        None
    }

    pub async fn handle_client(&self, mut stream: TcpStream) {
        let reader = std::io::BufReader::new(stream.try_clone().unwrap());
        let mut lines_iter = reader.lines();

        let user = match self.handle_pre_auth(&mut lines_iter) {
            Some(u) => u,
            None => return,
        };

        let incoming = self.chat.subscribe(&user);

        {
            let handler = self.clone();
            let user = user.clone();
            let mut stream = stream.try_clone().unwrap();
            std::thread::spawn(move || {
                for msg in futures::executor::block_on_stream(incoming) {
                    match msg {
                        ChatServiceEvent::Message(msg) => {
                            // Skip messages from self
                            if msg.get_user() == user {
                                continue;
                            }
                            handler.message_channel(
                                &mut stream,
                                msg.get_user(),
                                msg.get_channel(),
                                msg.get_content(),
                            );
                        }
                        ChatServiceEvent::JoinChannel(user, channel) => {
                            handler.announce_join(&mut stream, &user, &channel)
                        }
                        _ => continue,
                    }
                }
            });
        }

        self.welcome(&mut stream, &user, "Welcome to you who are already here!");

        for line in lines_iter {
            let line = line.unwrap();

            let msg = IrcMessage::from_str(&line);
            if let Err(m) = msg {
                println!("unknown command: {:?}", m);
                return;
            }

            match msg.unwrap() {
                IrcMessage::Ping(server) => {
                    self.pong(&mut stream, server);
                }
                IrcMessage::Join(channel) => {
                    let channel = channel.trim_left_matches("#");
                    self.chat.join(&user, channel);
                    let members = self.chat.get_members(channel);
                    self.announce_channel(&mut stream, &user, &members, channel);
                }
                IrcMessage::Mode(channel) => {
                    self.channel_mode(&mut stream, &user, channel);
                }
                IrcMessage::PrivMsg(target, content) => {
                    let mut message = Message::new();
                    message.set_user(user.to_string());
                    message.set_content(content.to_string());
                    message.set_channel(target.trim_left_matches("#").to_string());
                    self.chat.send(message);
                }
                _ => return,
            }
        }
    }

    fn message_channel(
        &self,
        stream: &mut dyn std::io::Write,
        user: &str,
        channel: &str,
        message: &str,
    ) {
        stream
            .write(format!(":{} PRIVMSG #{} :{}\n", user, channel, message).as_bytes())
            .unwrap();
    }

    fn welcome(&self, stream: &mut dyn std::io::Write, user: &str, msg: &str) {
        stream
            .write(format!(":localhost 001 {} :{}\n", user, msg).as_bytes())
            .unwrap();
    }

    fn pong(&self, stream: &mut dyn std::io::Write, server: &str) {
        stream.write("PONG :localhost\n".as_bytes()).unwrap();
    }

    fn announce_channel(
        &self,
        stream: &mut dyn std::io::Write,
        user: &str,
        members: &[String],
        channel: &str,
    ) {
        self.announce_join(stream, user, channel);
        self.channel_mode(stream, user, channel);
        stream
            .write(format!(":localhost 332 #{} :{}\n", channel, "no topic").as_bytes())
            .unwrap();
        for member in members {
            stream
                .write(format!(":localhost 353 {} = #{} :@{}\n", user, channel, member).as_bytes())
                .unwrap();
        }
        stream
            .write(
                format!(
                    ":localhost 366 {} #{} :End of /NAMES list.\n",
                    user, channel
                )
                .as_bytes(),
            )
            .unwrap();
    }

    fn announce_join(&self, stream: &mut dyn std::io::Write, user: &str, channel: &str) {
        stream
            .write(format!(":{} JOIN :#{}\n", user, channel).as_bytes())
            .unwrap();
    }

    fn channel_mode(&self, stream: &mut dyn std::io::Write, user: &str, channel: &str) {
        stream.write(format!(":localhost 324 {} #{} +nt\n", user, channel).as_bytes());
    }
}
