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

#[derive(Clone)]
pub struct QueueConsumer {
    queue_client: QueueClient,
    lockserv_client: lockserv_client::LockservClient,
}

impl QueueConsumer {
    pub fn new(
        queue_client: QueueClient,
        lockserv_client: lockserv_client::LockservClient,
    ) -> Self {
        Self {
            queue_client,
            lockserv_client,
        }
    }

    pub fn consume<F: Fn(&Message) -> Result<(), ()>>(&self, queue: String, consumer: F) {
        loop {
            if let Some(mut m) = self.queue_client.consume(queue.clone()) {
                // First, attempt to acquire a lock on the message and mark it as started.
                if let Err(_) = self
                    .lockserv_client
                    .acquire(message_to_lockserv_path(m.get_id()))
                {
                    continue;
                }
                m.set_status(Status::STARTED);
                self.queue_client.update(m.clone());

                let result = (consumer)(&m);

                // Re-assert lock ownership before writing completion status
                if let Err(_) = self
                    .lockserv_client
                    .reacquire(message_to_lockserv_path(m.get_id()))
                {
                    continue;
                }

                m.set_status(match result {
                    Ok(_) => Status::SUCCESS,
                    Err(_) => Status::FAILURE,
                });
                self.queue_client.update(m.clone());
            }

            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }
}
