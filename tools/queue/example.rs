use lockserv_client::*;
use queue_client::*;

use std::future::Future;
use std::pin::Pin;

#[derive(Clone)]
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

    fn resume(&self, message: &Message) -> Pin<Box<dyn Future<Output = ConsumeResult> + Send>> {
        let mut output = ArtifactsBuilder::new();
        output.add_string("signature", "sha1234".to_string());

        println!("finished task");

        Box::pin(std::future::ready(ConsumeResult::Success(output.build())))
    }

    fn consume(&self, message: &Message) -> Pin<Box<dyn Future<Output = ConsumeResult> + Send>> {
        let _self = self.clone();
        let message = message.clone();
        Box::pin(async move {
            let mut blockers = Vec::new();
            println!("consumed task");

            // Queue up a build
            let mut msg = Message::new();
            msg.name = "test r/123".to_string();
            let mut args = ArtifactsBuilder::new();
            args.add_string("path", "/var/log/syslog.0.dmesg".to_string());
            args.add_int("log_level", 5);
            msg.arguments = args.build();
            msg.blocks.id = message.id;
            msg.blocks.queue = message.queue.to_string();
            let id = _self
                .get_queue_client()
                .enqueue(String::from("builds"), msg)
                .await
                .unwrap();
            let mut b = BlockingMessage::new();
            b.id = id;
            b.queue = String::from("builds");
            blockers.push(b);

            // Queue up another build
            let mut msg = Message::new();
            msg.name = "test r/123".to_string();
            let mut args = ArtifactsBuilder::new();
            args.add_string("test", "//tools/queue:html_test".to_string());
            args.add_int("log_level", 3);
            msg.arguments = args.build();
            msg.blocks.id = message.id;
            msg.blocks.queue = message.queue.to_string();
            let id = _self
                .get_queue_client()
                .enqueue(String::from("builds"), msg)
                .await
                .unwrap();
            let mut b = BlockingMessage::new();
            b.id = id;
            b.queue = String::from("builds");
            blockers.push(b);

            let mut output = ArtifactsBuilder::new();
            output.add_string("targets", "asdf".to_string());
            output.add_string("targets", "fdsa".to_string());

            ConsumeResult::Blocked(output.build(), blockers)
        })
    }
}

#[derive(Clone)]
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

    fn consume(&self, message: &Message) -> Pin<Box<dyn Future<Output = ConsumeResult> + Send>> {
        let message = message.clone();
        Box::pin(async move {
            println!("presubmit: {:?}", message);
            tokio::time::delay_for(std::time::Duration::from_secs(10)).await;
            println!("done!");

            let mut output = ArtifactsBuilder::new();
            output.add_string("build_path", "/tmp/sha256/klog.jar".to_string());

            ConsumeResult::Success(output.build())
        })
    }
}

#[tokio::main]
async fn main() {
    tokio::spawn(Box::pin(async {
        let q = QueueClient::new("127.0.0.1", 5554);
        loop {
            let mut msg = Message::new();
            msg.name = "build r/123".to_string();

            let mut args = ArtifactsBuilder::new();
            args.add_string("path", "/var/log/syslog.0.dmesg".to_string());
            args.add_int("log_level", 5);
            msg.arguments = args.build();

            q.enqueue(String::from("presubmit"), msg).await;
            println!("enqueued task");
            tokio::time::delay_for(std::time::Duration::from_secs(5)).await;
        }
    }));

    tokio::spawn(Box::pin(async {
        let q = QueueClient::new("127.0.0.1", 5554);
        let ls = LockservClient::new("127.0.0.1", 5555);

        let consumer = TestConsumer::new(q, ls);
        consumer.start(String::from("builds")).await;
    }));

    let q = QueueClient::new("127.0.0.1", 5554);
    let ls = LockservClient::new("127.0.0.1", 5555);

    let consumer = PresubmitConsumer::new(q, ls);
    consumer.start(String::from("presubmit")).await;
}
