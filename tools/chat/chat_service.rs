use chat_grpc_rust::*;

use futures::stream::Stream;
use futures::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub enum ChatServiceEvent {
    Message(Message),
    JoinChannel(String, String),
    LeaveChannel(String, String),
}

#[derive(Clone)]
pub struct ChatServiceHandler {
    sender: UnboundedSender<ChatServiceEvent>,
    users: Arc<RwLock<HashMap<String, Vec<UnboundedSender<ChatServiceEvent>>>>>,
    channels: Arc<RwLock<HashMap<String, HashSet<String>>>>,
}

impl ChatServiceHandler {
    pub fn new() -> Self {
        let (sender, receiver) = futures::sync::mpsc::unbounded();
        let out = Self {
            sender,
            channels: Arc::new(RwLock::new(HashMap::new())),
            users: Arc::new(RwLock::new(HashMap::new())),
        };
        out
    }

    pub fn subscribe(&self, user: &str) -> UnboundedReceiver<ChatServiceEvent> {
        let (sender, reciever) = futures::sync::mpsc::unbounded();
        let mut us = self.users.write().unwrap();
        if let Some(u) = us.get_mut(user) {
            u.push(sender);
        } else {
            us.insert(user.to_string(), vec![sender]);
        }
        reciever
    }

    pub fn join(&self, user: &str, channel: &str) {
        self.announce(
            ChatServiceEvent::JoinChannel(user.to_owned(), channel.to_owned()),
            channel,
        );

        let mut ch = self.channels.write().unwrap();
        if let Some(c) = ch.get_mut(channel) {
            c.insert(user.to_string());
        } else {
            let mut h = HashSet::new();
            h.insert(user.to_string());
            ch.insert(channel.to_string(), h);
        }
    }

    pub fn get_members(&self, channel: &str) -> Vec<String> {
        let mut ch = self.channels.read().unwrap();
        if let Some(c) = ch.get(channel) {
            let mut out = Vec::new();
            for member in c {
                out.push(member.to_owned());
            }
            return out;
        }

        Vec::new()
    }

    pub fn announce(&self, event: ChatServiceEvent, channel: &str) {
        if let Some(users) = self.channels.read().unwrap().get(channel) {
            for user in users {
                if let Some(subscribers) = self.users.read().unwrap().get(user) {
                    for subscriber in subscribers {
                        subscriber.unbounded_send(event.clone());
                    }
                }
            }
        }
    }

    pub fn send(&self, message: Message) {
        let channel = message.get_channel().to_owned();
        self.announce(ChatServiceEvent::Message(message), &channel);
    }
}

impl ChatService for ChatServiceHandler {
    fn read(
        &self,
        _m: grpc::RequestOptions,
        mut req: ReadRequest,
    ) -> grpc::SingleResponse<ReadResponse> {
        grpc::SingleResponse::completed(ReadResponse::new())
    }

    fn send(
        &self,
        _m: grpc::RequestOptions,
        mut req: Message,
    ) -> grpc::SingleResponse<SendResponse> {
        self.send(req);
        grpc::SingleResponse::completed(SendResponse::new())
    }
}
