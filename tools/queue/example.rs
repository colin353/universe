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
        std::thread::sleep(std::time::Duration::from_secs(1));
        println!("done!");

        let mut output = ArtifactsBuilder::new();
        output.add_string("build_path", "/tmp/sha256/klog.jar".to_string());

        ConsumeResult::Success(output.build())
    }
}

fn main() {
    std::thread::spawn(|| {
        let q = QueueClient::new("127.0.0.1", 5554);
        loop {
            let mut msg = Message::new();
            msg.set_name("build r/123".to_string());

            let mut args = ArtifactsBuilder::new();
            args.add_string("path", "/var/log/syslog.0.dmesg".to_string());
            args.add_int("log_level", 5);
            *msg.mut_arguments() = protobuf::RepeatedField::from_vec(args.build());

            q.enqueue(String::from("builds"), msg);
            std::thread::sleep(std::time::Duration::from_secs(5));
        }
    });

    let q = QueueClient::new("127.0.0.1", 5554);
    let ls = LockservClient::new("127.0.0.1", 5555);

    let consumer = TestConsumer::new(q, ls);
    consumer.start(String::from("builds"));
}
