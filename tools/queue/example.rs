use lockserv_client::*;
use queue_client::*;

fn task(msg: &Message) -> Result<(), ()> {
    println!("got: {:?}", msg);
    Ok(())
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

    let consumer = QueueConsumer::new(q, ls);
    consumer.consume(String::from("/asdf"), task);
}
