use grpc::ClientStubExt;
pub use queue_grpc_rust::*;

use std::sync::Arc;

#[derive(Clone)]
pub struct QueueClient {
    client: Arc<QueueServiceClient>,
}

impl QueueClient {
    pub fn new(hostname: &str, port: u16) -> Self {
        Self {
            client: Arc::new(
                QueueServiceClient::new_plain(hostname, port, Default::default()).unwrap(),
            ),
        }
    }

    pub fn enqueue<T: protobuf::Message>(&self, queue: String, message: &T) {
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

    pub fn consume(&self, queue: String) -> Option<Message> {
        let mut req = ConsumeRequest::new();
        req.set_queue(queue);

        let mut response = self
            .client
            .consume(Default::default(), req.clone())
            .wait()
            .unwrap()
            .1;
        if response.get_message_available() {
            return Some(response.take_msg());
        }

        None
    }
}

pub fn message_to_lockserv_path(id: u64) -> String {
    format!("/ls/queue/{}", id)
}

pub enum ConsumeResult {
    Success(Vec<Artifact>),
    Failure(Vec<Artifact>),
    Blocked(Vec<Artifact>, Vec<u64>),
}

pub trait Consumer {
    fn consume(&self, message: &Message) -> ConsumeResult;

    fn get_queue_client(&self) -> &QueueClient;
    fn get_lockserv_client(&self) -> &lockserv_client::LockservClient;

    fn start(&self, queue: String) {
        loop {
            if let Some(mut m) = self.get_queue_client().consume(queue.clone()) {
                // First, attempt to acquire a lock on the message and mark it as started.
                if let Err(_) = self
                    .get_lockserv_client()
                    .acquire(message_to_lockserv_path(m.get_id()))
                {
                    continue;
                }
                m.set_status(Status::STARTED);
                self.get_queue_client().update(m.clone());

                // Run potentially long-running consume task.
                let panic_result =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.consume(&m)));

                if let Err(_) = panic_result {
                    // There was a panic. Just continue consuming. Queue service will
                    // retry this task if necessary.
                    println!("caught panic!");
                    continue;
                }

                let result = panic_result.unwrap();

                // Re-assert lock ownership before writing completion status
                if let Err(_) = self
                    .get_lockserv_client()
                    .reacquire(message_to_lockserv_path(m.get_id()))
                {
                    continue;
                }

                match result {
                    ConsumeResult::Success(results) => {
                        m.set_status(Status::SUCCESS);
                        for result in results {
                            m.mut_results().push(result);
                        }
                    }
                    ConsumeResult::Failure(results) => {
                        m.set_status(Status::FAILURE);
                        for result in results {
                            m.mut_results().push(result);
                        }
                    }
                    ConsumeResult::Blocked(results, blocked_by) => {
                        m.set_status(Status::BLOCKED);
                        for result in results {
                            m.mut_results().push(result);
                        }
                        m.set_blocked_by(blocked_by);
                    }
                };

                self.get_queue_client().update(m.clone());
            }

            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }
}
