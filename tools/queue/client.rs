use futures::StreamExt;
use grpc::ClientStubExt;
pub use queue_grpc_rust::*;

use std::sync::Arc;

#[derive(Clone)]
pub struct QueueClient {
    client: Arc<QueueServiceClient>,
}

pub fn get_timestamp_usec() -> u64 {
    let now = std::time::SystemTime::now();
    let since_epoch = now.duration_since(std::time::UNIX_EPOCH).unwrap();
    (since_epoch.as_secs() as u64) * 1_000_000 + (since_epoch.subsec_nanos() / 1000) as u64
}

fn wait<T: Send + Sync>(resp: grpc::SingleResponse<T>) -> Result<T, grpc::Error> {
    futures::executor::block_on(resp.join_metadata_result()).map(|r| r.1)
}

impl QueueClient {
    pub fn new(hostname: &str, port: u16) -> Self {
        let mut retries = 0;
        let client = loop {
            if let Ok(c) = QueueServiceClient::new_plain(hostname, port, Default::default()) {
                break c;
            }
            std::thread::sleep(std::time::Duration::from_secs(5));
            retries += 1;
            if retries > 10 {
                panic!("couldn't connect to queue service!");
            }
        };

        Self {
            client: Arc::new(client),
        }
    }

    pub fn enqueue(&self, queue: String, msg: Message) -> u64 {
        let mut req = EnqueueRequest::new();
        req.set_queue(queue);

        *req.mut_msg() = msg;

        let response = wait(self.client.enqueue(Default::default(), req)).unwrap();
        response.get_id()
    }

    pub fn read(&self, queue: String, id: u64) -> Option<Message> {
        let mut req = ReadRequest::new();
        req.set_queue(queue);
        req.set_id(id);

        let mut response = wait(self.client.read(Default::default(), req)).unwrap();

        if response.get_found() {
            Some(response.take_msg())
        } else {
            None
        }
    }

    pub fn enqueue_proto<T: protobuf::Message>(&self, queue: String, message: &T) -> u64 {
        let mut req = EnqueueRequest::new();
        req.set_queue(queue);

        let mut data = Vec::new();
        message.write_to_vec(&mut data);
        req.mut_msg().set_protobuf(data);

        let response = wait(self.client.enqueue(Default::default(), req)).unwrap();
        response.get_id()
    }

    pub fn update(&self, message: Message) -> Result<(), ()> {
        match wait(self.client.update(Default::default(), message)) {
            Ok(_) => Ok(()),
            Err(_) => Err(()),
        }
    }

    pub fn consume(&self, queue: String) -> Vec<Message> {
        let mut req = ConsumeRequest::new();
        req.set_queue(queue);

        let mut response = match wait(self.client.consume(Default::default(), req.clone())) {
            Ok(r) => r,
            Err(_) => return Vec::new(),
        };

        response.take_messages().into_vec()
    }

    pub fn consume_stream(&self, queue: String) -> Vec<Message> {
        let mut req = ConsumeRequest::new();
        req.set_queue(queue);

        let mut result = match futures::executor::block_on(
            self.client
                .consume_stream(Default::default(), req.clone())
                .drop_metadata()
                .take(1)
                .next(),
        ) {
            Some(Ok(r)) => r,
            _ => return Vec::new(),
        };

        return result.take_messages().into_vec();
    }
}

pub fn get_string_result<'a>(name: &str, m: &'a Message) -> Option<&'a str> {
    for arg in m.get_results() {
        if arg.get_name() == name {
            return Some(arg.get_value_string());
        }
    }
    None
}

pub fn get_string_arg<'a>(name: &str, m: &'a Message) -> Option<&'a str> {
    for arg in m.get_arguments() {
        if arg.get_name() == name {
            return Some(arg.get_value_string());
        }
    }
    None
}

pub fn get_int_arg<'a>(name: &str, m: &'a Message) -> Option<i64> {
    for arg in m.get_arguments() {
        if arg.get_name() == name {
            return Some(arg.get_value_int());
        }
    }
    None
}

pub fn get_bool_arg<'a>(name: &str, m: &'a Message) -> Option<bool> {
    for arg in m.get_arguments() {
        if arg.get_name() == name {
            return Some(arg.get_value_bool());
        }
    }
    None
}

pub fn get_float_arg<'a>(name: &str, m: &'a Message) -> Option<f32> {
    for arg in m.get_arguments() {
        if arg.get_name() == name {
            return Some(arg.get_value_float());
        }
    }
    None
}

pub fn message_to_lockserv_path(m: &Message) -> String {
    format!("/ls/queue/{}/{}", m.get_queue(), m.get_id())
}

pub enum ConsumeResult {
    Success(Vec<Artifact>),
    Failure(String, Vec<Artifact>),
    Blocked(Vec<Artifact>, Vec<BlockingMessage>),
}

pub fn next_state(mut x: u64) -> u64 {
    x ^= x << 13;
    x ^= x >> 17;
    x ^ x << 5
}

pub trait Consumer {
    fn consume(&self, message: &Message) -> ConsumeResult;

    fn resume(&self, message: &Message) -> ConsumeResult {
        self.consume(message)
    }

    fn get_queue_client(&self) -> &QueueClient;
    fn get_lockserv_client(&self) -> &lockserv_client::LockservClient;

    fn start(&self, queue: String) {
        let renewer_client = self.get_lockserv_client().clone();
        std::thread::spawn(move || {
            renewer_client.defend();
        });

        let mut prev_state = 0;
        let mut state = 0;

        loop {
            for mut m in self.get_queue_client().consume_stream(queue.clone()) {
                state = m.get_id();

                // First, attempt to acquire a lock on the message and mark it as started.
                let lock = match self
                    .get_lockserv_client()
                    .acquire(message_to_lockserv_path(&m))
                {
                    Ok(l) => l,
                    Err(_) => continue,
                };

                state = next_state(state);

                let did_resume = m.get_status() == Status::CONTINUE;
                if !did_resume {
                    m.set_start_time(get_timestamp_usec());
                }
                m.set_status(Status::STARTED);

                if let Err(_) = self.get_queue_client().update(m.clone()) {
                    continue;
                }

                state = next_state(state);

                self.get_lockserv_client().put_lock(lock);

                // Run potentially long-running consume task.
                let panic_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    if did_resume {
                        self.resume(&m)
                    } else {
                        self.consume(&m)
                    }
                }));

                if let Err(_) = panic_result {
                    // There was a panic. Just continue consuming. Queue service will
                    // retry this task if necessary.
                    println!("caught panic!");
                    continue;
                }

                state = next_state(state);

                // Let's grab the lock from the renewal thread
                let lock = match self
                    .get_lockserv_client()
                    .take_lock(&message_to_lockserv_path(&m))
                {
                    Some(l) => l,
                    None => {
                        println!("lock renewal thread failed!");
                        continue;
                    }
                };

                state = next_state(state);

                let result = panic_result.unwrap();

                // Re-assert lock ownership before writing completion status
                let lock = match self.get_lockserv_client().reacquire(lock) {
                    Ok(l) => l,
                    Err(_) => continue,
                };

                state = next_state(state);

                match result {
                    ConsumeResult::Success(results) => {
                        m.set_status(Status::SUCCESS);
                        m.set_end_time(get_timestamp_usec());
                        for result in results {
                            m.mut_results().push(result);
                        }
                    }
                    ConsumeResult::Failure(reason, results) => {
                        if !reason.is_empty() {
                            m.set_reason(reason);
                        }
                        m.set_status(Status::FAILURE);
                        m.set_end_time(get_timestamp_usec());
                        for result in results {
                            m.mut_results().push(result);
                        }
                    }
                    ConsumeResult::Blocked(results, blocked_by) => {
                        m.set_status(Status::BLOCKED);
                        for result in results {
                            m.mut_results().push(result);
                        }
                        for blocking in blocked_by {
                            m.mut_blocked_by().push(blocking);
                        }
                    }
                };
                if let Err(_) = self.get_queue_client().update(m.clone()) {
                    continue;
                };
                state = next_state(state);

                self.get_lockserv_client().yield_lock(lock);
            }

            // If the state didn't change since the last try, it means that the same operation
            // failed at the same step, so wait a bit to throttle requests
            if state == prev_state {
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
            prev_state = state;
        }
    }
}

pub struct ArtifactsBuilder {
    args: Vec<Artifact>,
}

impl ArtifactsBuilder {
    pub fn new() -> Self {
        Self { args: Vec::new() }
    }

    pub fn add_string(&mut self, name: &str, value: String) {
        let mut a = Artifact::new();
        a.set_name(name.to_owned());
        a.set_value_string(value);
        self.args.push(a)
    }

    pub fn add_int(&mut self, name: &str, value: i64) {
        let mut a = Artifact::new();
        a.set_name(name.to_owned());
        a.set_value_int(value);
        self.args.push(a)
    }

    pub fn add_float(&mut self, name: &str, value: f32) {
        let mut a = Artifact::new();
        a.set_name(name.to_owned());
        a.set_value_float(value);
        self.args.push(a)
    }

    pub fn add_bool(&mut self, name: &str, value: bool) {
        let mut a = Artifact::new();
        a.set_name(name.to_owned());
        a.set_value_bool(value);
        self.args.push(a)
    }

    pub fn build_rf(self) -> protobuf::RepeatedField<Artifact> {
        protobuf::RepeatedField::from_vec(self.build())
    }

    pub fn build(self) -> Vec<Artifact> {
        self.args
    }
}
