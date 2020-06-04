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

    pub fn wait_for_connection(&self) {
        let mut req = ReadRequest::new();
        for _ in 0..10 {
            if let Ok(_) = self.client.read(Default::default(), req.clone()).wait() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_secs(5));
        }

        panic!("Couldn't connect to queue service!");
    }

    pub fn enqueue(&self, queue: String, msg: Message) -> u64 {
        let mut req = EnqueueRequest::new();
        req.set_queue(queue);

        *req.mut_msg() = msg;

        let response = self
            .client
            .enqueue(Default::default(), req)
            .wait()
            .unwrap()
            .1;
        response.get_id()
    }

    pub fn read(&self, queue: String, id: u64) -> Option<Message> {
        let mut req = ReadRequest::new();
        req.set_queue(queue);
        req.set_id(id);

        let mut response = self.client.read(Default::default(), req).wait().unwrap().1;

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

        let response = self
            .client
            .enqueue(Default::default(), req)
            .wait()
            .unwrap()
            .1;
        response.get_id()
    }

    pub fn update(&self, message: Message) -> Result<(), ()> {
        match self.client.update(Default::default(), message).wait() {
            Ok(_) => Ok(()),
            Err(_) => Err(()),
        }
    }

    pub fn consume(&self, queue: String) -> Vec<Message> {
        let mut req = ConsumeRequest::new();
        req.set_queue(queue);

        let mut response = match self.client.consume(Default::default(), req.clone()).wait() {
            Ok(r) => r.1,
            Err(_) => return Vec::new(),
        };

        response.take_messages().into_vec()
    }

    pub fn consume_stream(&self, queue: String) -> Vec<Message> {
        let mut req = ConsumeRequest::new();
        req.set_queue(queue);

        let iter = match self
            .client
            .consume_stream(Default::default(), req.clone())
            .wait()
        {
            Ok(r) => r.1,
            Err(_) => return Vec::new(),
        };

        for result in iter {
            match result {
                Ok(mut r) => return r.take_messages().into_vec(),
                Err(_) => break,
            }
        }

        Vec::new()
    }
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

pub trait Consumer {
    fn consume(&self, message: &Message) -> ConsumeResult;

    fn resume(&self, message: &Message) -> ConsumeResult {
        self.consume(message)
    }

    fn get_queue_client(&self) -> &QueueClient;
    fn get_lockserv_client(&self) -> &lockserv_client::LockservClient;

    fn start(&self, queue: String) {
        // Wait for the queue server to start
        self.get_queue_client().wait_for_connection();

        let renewer_client = self.get_lockserv_client().clone();
        std::thread::spawn(move || {
            renewer_client.defend();
        });

        loop {
            for mut m in self.get_queue_client().consume_stream(queue.clone()) {
                // First, attempt to acquire a lock on the message and mark it as started.
                let lock = match self
                    .get_lockserv_client()
                    .acquire(message_to_lockserv_path(&m))
                {
                    Ok(l) => l,
                    Err(_) => continue,
                };

                let did_resume = m.get_status() == Status::CONTINUE;
                if !did_resume {
                    m.set_start_time(get_timestamp_usec());
                }
                m.set_status(Status::STARTED);

                if let Err(_) = self.get_queue_client().update(m.clone()) {
                    continue;
                }
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

                self.get_lockserv_client().yield_lock(lock);

                break;
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
