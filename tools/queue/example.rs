use lockserv_client::*;
use queue_client::*;

struct PresubmitConsumer {
    queue_client: QueueClient,
    lockserv_client: LockservClient,
}

impl PresubmitConsumer {
    fn new(queue_client: QueueClient, lockserv_client: LockservClient) -> Self {
        Self {
            queue_client,
            lockserv_client,
        }
    }
}

impl Consumer for PresubmitConsumer {
    fn get_queue_client(&self) -> &QueueClient {
        &self.queue_client
    }

    fn get_lockserv_client(&self) -> &LockservClient {
        &self.lockserv_client
    }

    fn resume(&self, message: &Message) -> ConsumeResult {
        let mut output = ArtifactsBuilder::new();
        output.add_string("signature", "sha1234".to_string());

        ConsumeResult::Success(output.build())
    }

    fn consume(&self, message: &Message) -> ConsumeResult {
        let mut blockers = Vec::new();

        // Queue up a build
        let mut msg = Message::new();
        msg.set_name("test r/123".to_string());
        let mut args = ArtifactsBuilder::new();
        args.add_string("path", "/var/log/syslog.0.dmesg".to_string());
        args.add_int("log_level", 5);
        *msg.mut_arguments() = protobuf::RepeatedField::from_vec(args.build());
        msg.mut_blocks().set_id(message.get_id());
        msg.mut_blocks().set_queue(message.get_queue().to_string());
        let id = self.get_queue_client().enqueue(String::from("builds"), msg);
        let mut b = BlockingMessage::new();
        b.set_id(id);
        b.set_queue(String::from("builds"));
        blockers.push(b);

        // Queue up another build
        let mut msg = Message::new();
        msg.set_name("test r/123".to_string());
        let mut args = ArtifactsBuilder::new();
        args.add_string("test", "//tools/queue:html_test".to_string());
        args.add_int("log_level", 3);
        *msg.mut_arguments() = protobuf::RepeatedField::from_vec(args.build());
        msg.mut_blocks().set_id(message.get_id());
        msg.mut_blocks().set_queue(message.get_queue().to_string());
        let id = self.get_queue_client().enqueue(String::from("builds"), msg);
        let mut b = BlockingMessage::new();
        b.set_id(id);
        b.set_queue(String::from("builds"));
        blockers.push(b);

        let mut output = ArtifactsBuilder::new();
        output.add_string("targets", "asdf".to_string());
        output.add_string("targets", "fdsa".to_string());

        ConsumeResult::Blocked(output.build(), blockers)
    }
}

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
        println!("presubmit: {:?}", message);
        std::thread::sleep(std::time::Duration::from_secs(10));
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

            q.enqueue(String::from("presubmit"), msg);
            std::thread::sleep(std::time::Duration::from_secs(5));
        }
    });

    std::thread::spawn(|| {
        let q = QueueClient::new("127.0.0.1", 5554);
        let ls = LockservClient::new("127.0.0.1", 5555);

        let consumer = TestConsumer::new(q, ls);
        consumer.start(String::from("builds"));
    });

    let q = QueueClient::new("127.0.0.1", 5554);
    let ls = LockservClient::new("127.0.0.1", 5555);

    let consumer = PresubmitConsumer::new(q, ls);
    consumer.start(String::from("presubmit"));
}
