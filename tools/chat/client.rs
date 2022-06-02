pub use chat_grpc_rust::*;
use grpc::ClientStub;
use grpc::ClientStubExt;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct ChatClient {
    hostname: String,
    port: u16,
    client: Arc<Mutex<Option<ChatServiceClient>>>,
    stub: bool,
}

pub fn get_timestamp_usec() -> u64 {
    let now = std::time::SystemTime::now();
    let since_epoch = now.duration_since(std::time::UNIX_EPOCH).unwrap();
    (since_epoch.as_secs() as u64) * 1_000_000 + (since_epoch.subsec_nanos() / 1000) as u64
}

fn wait<T: Send + Sync>(resp: grpc::SingleResponse<T>) -> Result<T, grpc::Error> {
    futures::executor::block_on(resp.join_metadata_result()).map(|r| r.1)
}

impl ChatClient {
    pub fn new(hostname: &str, port: u16, use_tls: bool) -> Self {
        let mut out = Self {
            client: Arc::new(Mutex::new(None)),
            hostname: hostname.to_owned(),
            port,
            stub: false,
        };

        if let Ok(c) = out.make_client(use_tls) {
            (*out.client.lock().unwrap()) = Some(c);
        } else {
            eprintln!("couldn't reach chat service! proceeding without it");
        }
        out
    }

    pub fn new_stub() -> Self {
        Self {
            client: Arc::new(Mutex::new(None)),
            hostname: String::new(),
            port: 0,
            stub: true,
        }
    }

    fn make_client(&self, use_tls: bool) -> Result<ChatServiceClient, grpc::Error> {
        if use_tls {
            let grpc_client = grpc_tls::make_tls_client(&self.hostname, self.port);
            return Ok(ChatServiceClient::with_client(Arc::new(grpc_client)));
        }

        ChatServiceClient::new_plain(&self.hostname, self.port, Default::default())
    }

    pub fn send_message(&self, user: &str, channel: &str, content: String) {
        let mut msg = Message::new();
        msg.set_channel(channel.to_owned());
        msg.set_user(user.to_owned());
        msg.set_content(content);
        self.send(msg);
    }

    pub fn send(&self, mut message: Message) {
        if self.stub {
            return;
        }

        let mut maybe_client = self.client.lock().unwrap();
        if maybe_client.is_none() {
            eprintln!("attempting to connect to the chat service...");
            if let Ok(c) =
                ChatServiceClient::new_plain(&self.hostname, self.port, Default::default())
            {
                *maybe_client = Some(c);
            }
        }

        if maybe_client.is_none() {
            eprintln!("unable to reach chat service, dropping message");
            return;
        }

        if message.get_timestamp() == 0 {
            message.set_timestamp(get_timestamp_usec());
        }
        match wait(
            maybe_client
                .as_ref()
                .unwrap()
                .send(Default::default(), message),
        ) {
            Ok(_) => return,
            Err(e) => eprintln!("couldn't log chat! {:?}", e),
        };
    }
}
