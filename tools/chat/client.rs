pub use chat_grpc_rust::*;
use grpc::ClientStubExt;
use std::sync::Arc;

#[derive(Clone)]
pub struct ChatClient {
    client: Arc<ChatServiceClient>,
}

pub fn get_timestamp_usec() -> u64 {
    let now = std::time::SystemTime::now();
    let since_epoch = now.duration_since(std::time::UNIX_EPOCH).unwrap();
    (since_epoch.as_secs() as u64) * 1_000_000 + (since_epoch.subsec_nanos() / 1000) as u64
}

impl ChatClient {
    pub fn new(hostname: &str, port: u16) -> Self {
        let mut retries = 0;
        let client = loop {
            if let Ok(c) = ChatServiceClient::new_plain(hostname, port, Default::default()) {
                break c;
            }
            std::thread::sleep(std::time::Duration::from_secs(5));
            retries += 1;
            if retries > 10 {
                panic!("couldn't connect to chat service!");
            }
        };
        Self {
            client: Arc::new(client),
        }
    }

    pub fn send(&self, mut message: Message) {
        if message.get_timestamp() == 0 {
            message.set_timestamp(get_timestamp_usec());
        }
        let response = self
            .client
            .send(Default::default(), message)
            .wait()
            .unwrap()
            .1;
    }
}
