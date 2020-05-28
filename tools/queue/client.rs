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

impl QueueClient {
    pub fn new(hostname: &str, port: u16) -> Self {
        Self {
            client: Arc::new(
                QueueServiceClient::new_plain(hostname, port, Default::default()).unwrap(),
            ),
        }
    }

    pub fn enqueue(&self, queue: String, msg: Message) {
        let mut req = EnqueueRequest::new();
        req.set_queue(queue);

        *req.mut_msg() = msg;

        self.client.enqueue(Default::default(), req).wait().unwrap();
    }

    pub fn enqueue_proto<T: protobuf::Message>(&self, queue: String, message: &T) {
        let mut req = EnqueueRequest::new();
        req.set_queue(queue);

        let mut data = Vec::new();
        message.write_to_vec(&mut data);
        req.mut_msg().set_protobuf(data);

        self.client.enqueue(Default::default(), req).wait().unwrap();
    }

    pub fn update(&self, message: Message) {
        self.client
            .update(Default::default(), message)
            .wait()
            .unwrap();
    }

    pub fn consume(&self, queue: String) -> Vec<Message> {
        let mut req = ConsumeRequest::new();
        req.set_queue(queue);

        let mut response = self
            .client
            .consume(Default::default(), req.clone())
            .wait()
            .unwrap()
            .1;

        response.take_messages().into_vec()
    }
}

pub fn message_to_lockserv_path(m: &Message) -> String {
    format!("/ls/queue/{}/{}", m.get_queue(), m.get_id())
}

pub enum ConsumeResult {
    Success(Vec<Artifact>),
    Failure(Vec<Artifact>),
    Blocked(Vec<Artifact>, Vec<BlockingMessage>),
}

pub trait Consumer {
    fn consume(&self, message: &Message) -> ConsumeResult;

    fn get_queue_client(&self) -> &QueueClient;
    fn get_lockserv_client(&self) -> &lockserv_client::LockservClient;

    fn start(&self, queue: String) {
        let renewer_client = self.get_lockserv_client().clone();
        std::thread::spawn(move || {
            renewer_client.defend();
        });

        loop {
            for mut m in self.get_queue_client().consume(queue.clone()) {
                // First, attempt to acquire a lock on the message and mark it as started.
                let lock = match self
                    .get_lockserv_client()
                    .acquire(message_to_lockserv_path(&m))
                {
                    Ok(l) => l,
                    Err(_) => continue,
                };

                m.set_status(Status::STARTED);
                m.set_start_time(get_timestamp_usec());
                self.get_queue_client().update(m.clone());
                self.get_lockserv_client().put_lock(lock);

                // Run potentially long-running consume task.
                let panic_result =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.consume(&m)));

                if let Err(_) = panic_result {
                    // There was a panic. Just continue consuming. Queue service will
                    // retry this task if necessary.
                    println!("caught panic!");
                    continue;
                }

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

                let result = panic_result.unwrap();

                // Re-assert lock ownership before writing completion status
                let lock = match self.get_lockserv_client().reacquire(lock) {
                    Ok(l) => l,
                    Err(_) => continue,
                };

                match result {
                    ConsumeResult::Success(results) => {
                        m.set_status(Status::SUCCESS);
                        m.set_end_time(get_timestamp_usec());
                        for result in results {
                            m.mut_results().push(result);
                        }
                    }
                    ConsumeResult::Failure(results) => {
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
                self.get_queue_client().update(m.clone());

                self.get_lockserv_client().yield_lock(lock);
            }

            std::thread::sleep(std::time::Duration::from_secs(1));
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

    pub fn build(self) -> Vec<Artifact> {
        self.args
    }
}
