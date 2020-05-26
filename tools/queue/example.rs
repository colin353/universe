use lockserv_client::*;
use queue_client::*;

struct TestConsumer {
    queue_client: QueueClient,
    lockserv_client: LockservClient,
}

impl TestConsumer {
    fn new(queue_client: QueueClient, lockserv_client: LockservClient) -> Self {
        Self {
            queue_client,
            lockserv_client,
        }
    }
}

impl Consumer for TestConsumer {
    fn get_queue_client(&self) -> &QueueClient {
        &self.queue_client
    }

    fn get_lockserv_client(&self) -> &LockservClient {
        &self.lockserv_client
    }

    fn consume(&self, message: &Message) -> ConsumeResult {
        println!("got: {:?}", message);
        std::thread::sleep(std::time::Duration::from_secs(40));
        println!("done!");
        ConsumeResult::Success(Vec::new())
    }
}

fn main() {
    std::thread::spawn(|| {
        let q = QueueClient::new("127.0.0.1", 5554);
        loop {
            q.enqueue(String::from("/asdf"), &Artifact::new());
            std::thread::sleep(std::time::Duration::from_secs(5));
        }
    });

    let q = QueueClient::new("127.0.0.1", 5554);
    let ls = LockservClient::new("127.0.0.1", 5555);

    let consumer = TestConsumer::new(q, ls);
    consumer.start(String::from("/asdf"));
}
