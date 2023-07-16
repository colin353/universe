use futures::{FutureExt, StreamExt};
pub use queue_bus::*;

use std::future::Future;
use std::pin::Pin;

use std::sync::Arc;

#[derive(Clone)]
pub struct QueueClient {
    client: QueueAsyncClient,
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
        let connector = bus_rpc::HyperClient::new(hostname.to_string(), port);
        let client = QueueAsyncClient::new(Arc::new(connector));

        Self { client }
    }

    pub async fn enqueue(&self, queue: String, msg: Message) -> Result<u64, bus::BusRpcError> {
        let mut req = EnqueueRequest::new();
        req.queue = queue;

        req.msg = msg;

        let response = self.client.enqueue(req).await?;
        Ok(response.id)
    }

    pub async fn read(&self, queue: String, id: u64) -> Result<Option<Message>, bus::BusRpcError> {
        let mut req = ReadRequest::new();
        req.queue = queue;
        req.id = id;

        let response = self.client.read(req).await?;

        if response.found {
            Ok(Some(response.msg))
        } else {
            Ok(None)
        }
    }

    pub async fn update(&self, message: Message) -> Result<UpdateResponse, bus::BusRpcError> {
        self.client.update(message).await
    }

    pub async fn consume(&self, queue: String) -> Result<Vec<Message>, bus::BusRpcError> {
        let mut req = ConsumeRequest::new();
        req.queue = queue;

        Ok(self.client.consume(req.clone()).await?.messages)
    }

    pub async fn consume_stream(&self, queue: String) -> Result<Vec<Message>, bus::BusRpcError> {
        let mut req = ConsumeRequest::new();
        req.queue = queue;

        Ok(self
            .client
            .consume_stream(req.clone())
            .await?
            .next()
            .await
            .map(|m| m.messages)
            .unwrap_or_else(Vec::new))
    }
}

pub fn get_string_result<'a>(name: &str, m: &'a Message) -> Option<&'a str> {
    for arg in &m.results {
        if arg.name == name {
            return Some(&arg.value_string);
        }
    }
    None
}

pub fn get_string_arg<'a>(name: &str, m: &'a Message) -> Option<&'a str> {
    for arg in &m.arguments {
        if arg.name == name {
            return Some(&arg.value_string);
        }
    }
    None
}

pub fn get_int_arg(name: &str, m: &Message) -> Option<i64> {
    for arg in &m.arguments {
        if arg.name == name {
            return Some(arg.value_int);
        }
    }
    None
}

pub fn get_bool_arg(name: &str, m: &Message) -> Option<bool> {
    for arg in &m.arguments {
        if arg.name == name {
            return Some(arg.value_bool);
        }
    }
    None
}

pub fn get_float_arg(name: &str, m: &Message) -> Option<f32> {
    for arg in &m.arguments {
        if arg.name == name {
            return Some(arg.value_float);
        }
    }
    None
}

pub fn message_to_lockserv_path(m: &Message) -> String {
    format!("/ls/queue/{}/{}", m.queue, m.id)
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

pub trait Consumer: Clone + Send + Sync + 'static {
    fn consume(&self, message: &Message) -> Pin<Box<dyn Future<Output = ConsumeResult> + Send>>;

    fn resume(&self, message: &Message) -> Pin<Box<dyn Future<Output = ConsumeResult> + Send>> {
        self.consume(message)
    }

    fn get_queue_client(&self) -> &QueueClient;
    fn get_lockserv_client(&self) -> &lockserv_client::LockservClient;

    fn start(&self, queue: String) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        let _self = self.clone();
        Box::pin(async move {
            let renewer_client = _self.get_lockserv_client().clone();

            // Use tokio??
            std::thread::spawn(move || {
                renewer_client.defend();
            });

            let mut prev_state = 0;
            let mut state = 0;

            loop {
                let messages = match _self.get_queue_client().consume_stream(queue.clone()).await {
                    Ok(m) => m,
                    Err(_) => {
                        tokio::time::interval(std::time::Duration::from_secs(1))
                            .tick()
                            .await;
                        continue;
                    }
                };

                for mut m in messages {
                    state = m.id;

                    // First, attempt to acquire a lock on the message and mark it as started.
                    let lock = match _self
                        .get_lockserv_client()
                        .acquire(message_to_lockserv_path(&m))
                    {
                        Ok(l) => l,
                        Err(_) => continue,
                    };

                    state = next_state(state);

                    let did_resume = m.status == Status::Continue;
                    if !did_resume {
                        m.start_time = get_timestamp_usec();
                    }
                    m.status = Status::Started;

                    if let Err(_) = _self.get_queue_client().update(m.clone()).await {
                        continue;
                    }

                    state = next_state(state);

                    _self.get_lockserv_client().put_lock(lock);

                    // Run potentially long-running consume task.
                    let panic_result = if did_resume {
                        std::panic::AssertUnwindSafe(_self.resume(&m))
                            .catch_unwind()
                            .await
                    } else {
                        std::panic::AssertUnwindSafe(_self.consume(&m))
                            .catch_unwind()
                            .await
                    };

                    if let Err(_) = panic_result {
                        // There was a panic. Just continue consuming. Queue service will
                        // retry this task if necessary.
                        println!("caught panic!");
                        continue;
                    }

                    state = next_state(state);

                    // Let's grab the lock from the renewal thread
                    let lock = match _self
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
                    let lock = match _self.get_lockserv_client().reacquire(lock) {
                        Ok(l) => l,
                        Err(_) => continue,
                    };

                    state = next_state(state);

                    match result {
                        ConsumeResult::Success(results) => {
                            m.status = Status::Success;
                            m.end_time = get_timestamp_usec();
                            for result in results {
                                m.results.push(result);
                            }
                        }
                        ConsumeResult::Failure(reason, results) => {
                            if !reason.is_empty() {
                                m.reason = reason;
                            }
                            m.status = Status::Failure;
                            m.end_time = get_timestamp_usec();
                            for result in results {
                                m.results.push(result);
                            }
                        }
                        ConsumeResult::Blocked(results, blocked_by) => {
                            m.status = Status::Blocked;
                            for result in results {
                                m.results.push(result);
                            }
                            for blocking in blocked_by {
                                m.blocked_by.push(blocking);
                            }
                        }
                    };
                    if let Err(_) = _self.get_queue_client().update(m.clone()).await {
                        continue;
                    };
                    state = next_state(state);

                    _self.get_lockserv_client().yield_lock(lock);
                }

                // If the state didn't change since the last try, it means that the same operation
                // failed at the same step, so wait a bit to throttle requests
                if state == prev_state {
                    tokio::time::delay_for(std::time::Duration::from_secs(1)).await;
                }
                prev_state = state;
            }
        })
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
        a.name = name.to_owned();
        a.value_string = value;
        self.args.push(a)
    }

    pub fn add_int(&mut self, name: &str, value: i64) {
        let mut a = Artifact::new();
        a.name = name.to_owned();
        a.value_int = value;
        self.args.push(a)
    }

    pub fn add_float(&mut self, name: &str, value: f32) {
        let mut a = Artifact::new();
        a.name = name.to_owned();
        a.value_float = value;
        self.args.push(a)
    }

    pub fn add_bool(&mut self, name: &str, value: bool) {
        let mut a = Artifact::new();
        a.name = name.to_owned();
        a.value_bool = value;
        self.args.push(a)
    }

    pub fn build_rf(self) -> protobuf::RepeatedField<Artifact> {
        protobuf::RepeatedField::from_vec(self.build())
    }

    pub fn build(self) -> Vec<Artifact> {
        self.args
    }
}
