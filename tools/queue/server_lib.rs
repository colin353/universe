use bus::Deserialize;
use largetable_client::LargeTableClient;
use queue_bus::{
    ConsumeRequest, ConsumeResponse, EnqueueRequest, EnqueueResponse, Message,
    QueueAsyncServiceHandler, QueueWindowLimit, ReadRequest, ReadResponse, Status, UpdateResponse,
};
use queue_client::message_to_lockserv_path;

use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::iter::FromIterator;
use std::pin::Pin;
use std::sync::{Arc, RwLock};

use futures::channel::mpsc;
use futures::channel::mpsc::UnboundedSender;
use futures::StreamExt;

pub const QUEUE: &'static str = "queues";
pub const QUEUES: &'static str = "queues";
pub const MESSAGE_IDS: &'static str = "queue-ids";
pub const MAX_RETRIES: u64 = 3;

pub fn get_timestamp_usec() -> u64 {
    let now = std::time::SystemTime::now();
    let since_epoch = now.duration_since(std::time::UNIX_EPOCH).unwrap();
    (since_epoch.as_secs() as u64) * 1_000_000 + (since_epoch.subsec_nanos() / 1000) as u64
}

pub fn get_queues_rowname(queue: &str) -> String {
    format!("{}/{}", QUEUES, queue)
}

pub fn get_queue_rowname(queue: &str) -> String {
    format!("{}/{}", QUEUE, queue)
}

pub fn get_queue_window_rowname() -> String {
    format!("{}/limit", QUEUE)
}

pub fn get_colname(id: u64) -> String {
    format!("{:016x}", id)
}

fn is_consumable_status(s: Status) -> bool {
    s == Status::Created || s == Status::Retry || s == Status::Continue
}

fn is_bumpable_status(s: Status) -> bool {
    s == Status::Started || s == Status::Blocked
}

pub fn is_complete_status(s: Status) -> bool {
    s == Status::Success || s == Status::Failure
}

pub struct MessageRouter {
    listeners: RwLock<HashMap<String, Vec<UnboundedSender<Message>>>>,
}

impl MessageRouter {
    pub fn new() -> Self {
        Self {
            listeners: RwLock::new(HashMap::new()),
        }
    }

    fn subscribe(&self, queue: String, stream: UnboundedSender<Message>) {
        let mut l = self.listeners.write().unwrap();
        match l.get_mut(&queue) {
            Some(streams) => streams.push(stream),
            None => {
                l.insert(queue, vec![stream]);
            }
        };
    }

    fn put(&self, queue: &str, message: &Message) {
        let mut closed_streams = Vec::new();
        {
            let l = self.listeners.read().unwrap();
            let streams = match l.get(queue) {
                Some(streams) => streams,
                None => return,
            };

            for (index, stream) in streams.iter().enumerate() {
                if let Err(_) = stream.unbounded_send(message.clone()) {
                    closed_streams.push(index);
                }
            }
        }

        if closed_streams.len() > 0 {
            let mut l = self.listeners.write().unwrap();
            let streams = match l.get_mut(queue) {
                Some(streams) => streams,
                None => return,
            };

            // We need to recalculate the closed stream indices because we released the lock, and
            // somebody else might have modified the stream list in the meantime.
            closed_streams.clear();
            for (index, stream) in streams.iter().enumerate() {
                if stream.is_closed() {
                    closed_streams.push(index);
                }
            }

            // There's probably a way better way to do this, but I'm trying to remove a bunch of
            // closed streams from the streams array.
            for (num_removed, stream_index) in closed_streams.iter().enumerate() {
                streams.swap_remove(stream_index - num_removed);
            }
        }
    }
}

#[derive(Clone)]
pub struct QueueServiceHandler {
    database: LargeTableClient,
    lockserv_client: Option<lockserv_client::LockservClient>,
    queues: Arc<RwLock<HashSet<String>>>,
    router: Arc<MessageRouter>,
    base_url: String,
}

impl QueueServiceHandler {
    pub async fn new(
        database: largetable_client::LargeTableClient,
        lockserv_client: lockserv_client::LockservClient,
        base_url: String,
    ) -> std::io::Result<Self> {
        // TODO: Set up compaction policy
        /*
        let mut policy = largetable_client::CompactionPolicy::new();
        policy.set_row(QUEUE.to_owned());
        policy.set_scope(String::new());
        policy.set_history(1);
        database.set_compaction_policy(policy);
        */

        let iter = database
            .read_range(
                largetable_client::Filter {
                    row: QUEUES,
                    spec: "",
                    min: "",
                    max: "",
                },
                0,
                1000,
            )
            .await?
            .records
            .into_iter()
            .map(|r| r.key);

        let queues = HashSet::from_iter(iter);

        Ok(Self {
            database,
            queues: Arc::new(RwLock::new(queues)),
            lockserv_client: Some(lockserv_client),
            router: Arc::new(MessageRouter::new()),
            base_url,
        })
    }

    pub fn new_fake(database: LargeTableClient) -> Self {
        Self {
            database,
            queues: Arc::new(RwLock::new(HashSet::new())),
            lockserv_client: None,
            router: Arc::new(MessageRouter::new()),
            base_url: String::new(),
        }
    }

    pub async fn maybe_create_queue(&self, queue: &str) -> std::io::Result<()> {
        if self.queues.read().unwrap().contains(queue) {
            return Ok(());
        }

        // Update the cache
        {
            let mut queues = self.queues.write().unwrap();
            queues.insert(queue.to_string());
        }

        // Create the queue
        self.database
            .write(QUEUES.to_string(), queue.to_string(), 0, bus::Nothing {})
            .await?;

        Ok(())
    }

    pub async fn enqueue(&self, req: EnqueueRequest) -> std::io::Result<EnqueueResponse> {
        self.maybe_create_queue(&req.queue).await?;

        // First, reserve an ID for this task
        let id = self
            .database
            .reserve_id(MESSAGE_IDS.to_string(), req.queue.clone())
            .await?;

        let mut message = req.msg;
        message.id = id;
        message.status = Status::Created;
        message.queue = req.queue.to_owned();
        message.enqueued_time = get_timestamp_usec();

        self.update(message.clone()).await?;
        self.router.put(&req.queue, &message);

        let mut response = EnqueueResponse::new();
        response.id = id;
        Ok(response)
    }

    async fn get_queue_limit(&self, queue: &str) -> u64 {
        // If this is the oldest in-progress task, then bump up
        // the queue window.
        match self
            .database
            .read::<QueueWindowLimit>(&get_queue_window_rowname(), queue, 0)
            .await
        {
            Some(Ok(l)) => l.limit,
            None | Some(Err(_)) => 0,
        }
    }

    pub async fn update(&self, msg: Message) -> std::io::Result<UpdateResponse> {
        self.database
            .write(get_queue_rowname(&msg.queue), get_colname(msg.id), 0, &msg)
            .await?;

        Ok(UpdateResponse::new())
    }

    pub async fn read(&self, queue: &str, id: u64) -> std::io::Result<Option<Message>> {
        match self
            .database
            .read::<Message>(&get_queue_rowname(queue), &get_colname(id), 0)
            .await
        {
            Some(Ok(mut m)) => {
                m.info_url = format!("{}queue/{}/{}", self.base_url, queue, id);
                Ok(Some(m))
            }
            Some(Err(e)) => Err(e),
            None => Ok(None),
        }
    }

    pub async fn consume(&self, req: ConsumeRequest) -> std::io::Result<ConsumeResponse> {
        let limit = self.get_queue_limit(&req.queue).await;

        let iter = self
            .database
            .read_range(
                largetable_client::Filter {
                    row: &get_queue_rowname(&req.queue),
                    spec: "",
                    min: &get_colname(limit),
                    max: "",
                },
                0,
                1000,
            )
            .await?
            .records
            .into_iter()
            .map(|r| Message::decode(&r.data).unwrap());

        let mut limit_bump = 0;
        let mut all_complete = true;
        let mut response = ConsumeResponse::new();
        for msg in iter {
            if all_complete && is_complete_status(msg.status) {
                limit_bump += 1;
            } else {
                all_complete = false;
            }

            if is_consumable_status(msg.status) {
                response.messages.push(msg);
                if response.messages.len() > 10 {
                    break;
                }
            }
        }

        // Possibly update the bump limit
        if limit_bump > 0 {
            let mut new_limit = QueueWindowLimit::new();
            new_limit.limit = limit + limit_bump;
            self.database
                .write(get_queue_window_rowname(), req.queue, 0, &new_limit)
                .await?;
        }

        Ok(response)
    }

    // This method watches for changes that were started but timed out, and
    // puts them back onto the queue.
    pub async fn bump(&self) -> std::io::Result<()> {
        let queues: Vec<_> = self
            .database
            .read_range(
                largetable_client::Filter {
                    row: QUEUE,
                    spec: "",
                    min: "",
                    max: "",
                },
                0,
                1000,
            )
            .await?
            .records
            .into_iter()
            .map(|r| r.key)
            .collect();

        for queue in queues {
            self.bump_queue(queue).await?;
        }

        Ok(())
    }

    pub async fn bump_queue(&self, queue: String) -> std::io::Result<()> {
        let limit = self.get_queue_limit(&queue).await;

        let eligible_messages: Vec<_> = self
            .database
            .read_range(
                largetable_client::Filter {
                    row: &get_queue_rowname(&queue),
                    spec: "",
                    min: &get_colname(limit),
                    max: "",
                },
                0,
                1000,
            )
            .await?
            .records
            .into_iter()
            .map(|r| Message::decode(&r.data).unwrap())
            .filter(|m| is_bumpable_status(m.status))
            .collect();

        for message in eligible_messages {
            let lock = match self
                .lockserv_client
                .as_ref()
                .unwrap()
                .acquire(message_to_lockserv_path(&message))
                .await
            {
                Ok(l) => l,
                Err(_) => continue,
            };

            // Reload the message from the database now that we got the lock
            let mut message = match self.read(&queue, message.id).await? {
                Some(m) => m,
                None => continue,
            };

            if message.status == Status::Started {
                println!("message {} started but failed, retrying...", message.id);
                // We should never be able to acquire a lock on a started process, so
                // it must have failed.
                message.failures = message.failures + 1;

                if message.failures >= MAX_RETRIES {
                    message.status = Status::Failure;
                    message.reason = String::from("reached max retries");
                } else {
                    message.status = Status::Retry;
                }

                self.update(message.clone()).await?;

                if is_consumable_status(message.status) {
                    self.router.put(&queue, &message);
                }
            } else if message.status == Status::Blocked {
                // Check if blocked messages are unblocked yet, and return them to
                // the queue with CONTINUE status if they're unblocked.
                let mut blocked = false;
                let mut error = false;
                for blocker in &message.blocked_by {
                    let m = match self.read(&blocker.queue, blocker.id).await? {
                        Some(m) => m,
                        None => {
                            error = true;
                            break;
                        }
                    };
                    if !is_complete_status(m.status) {
                        blocked = true;
                        break;
                    }
                }

                if error {
                    message.status = Status::Failure;
                    message.reason = String::from("blocked by unknown message!");
                    self.update(message).await?;
                } else if !blocked {
                    message.status = Status::Continue;
                    self.update(message.clone()).await?;
                    self.router.put(&queue, &message);
                }
            }

            self.lockserv_client
                .as_ref()
                .unwrap()
                .yield_lock(lock)
                .await
                .unwrap();
        }

        Ok(())
    }
}

impl QueueAsyncServiceHandler for QueueServiceHandler {
    fn enqueue(
        &self,
        req: EnqueueRequest,
    ) -> Pin<Box<dyn Future<Output = Result<EnqueueResponse, bus::BusRpcError>> + Send>> {
        let _self = self.clone();
        Box::pin(async move { Ok(_self.enqueue(req).await?) })
    }

    fn update(
        &self,
        req: Message,
    ) -> Pin<Box<dyn Future<Output = Result<UpdateResponse, bus::BusRpcError>> + Send>> {
        let _self = self.clone();
        Box::pin(async move { Ok(_self.update(req).await?) })
    }

    fn consume(
        &self,
        req: ConsumeRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ConsumeResponse, bus::BusRpcError>> + Send>> {
        let _self = self.clone();
        Box::pin(async move { Ok(_self.consume(req).await?) })
    }

    fn read(
        &self,
        req: ReadRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ReadResponse, bus::BusRpcError>> + Send>> {
        let _self = self.clone();
        Box::pin(async move {
            let mut response = ReadResponse::new();
            let maybe_message = _self.read(&req.queue, req.id).await?;
            match maybe_message {
                Some(m) => {
                    response.found = true;
                    response.msg = m;
                }
                None => (),
            };
            Ok(response)
        })
    }

    fn consume_stream(
        &self,
        req: ConsumeRequest,
        sink: bus::BusSink<ConsumeResponse>,
    ) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        let (tx, mut rx) = mpsc::unbounded();
        self.router.subscribe(req.queue.to_owned(), tx);

        let _self = self.clone();
        Box::pin(async move {
            let initial_response = _self.consume(req).await.unwrap();
            if !initial_response.messages.is_empty() {
                sink.send(initial_response).await.unwrap();
            }
            while let Some(r) = rx.next().await {
                let mut msg = ConsumeResponse::new();
                msg.messages.push(r);
                if let Err(_) = sink.send(msg).await {
                    return;
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_handler() -> QueueServiceHandler<largetable_test::LargeTableMockClient> {
        let database = largetable_test::LargeTableMockClient::new();
        QueueServiceHandler::new_fake(database)
    }

    #[test]
    fn test_enqueue_reserve() {
        let q = make_handler();
        let mut req = EnqueueRequest::new();
        req.set_queue(String::from("test"));
        q.enqueue(req);

        let mut req = ConsumeRequest::new();
        req.set_queue(String::from("test"));
        let mut response = q.consume(req);
        assert_eq!(response.messages.len(), 1);

        // Now let's update it to be in progress
        let mut msg = &mut response.messages[0];
        msg.set_status(Status::Started);
        q.update(msg.clone());

        // There shouldn't be any more messages available
        let mut req = ConsumeRequest::new();
        req.set_queue(String::from("test"));
        let response = q.consume(req);
        assert_eq!(response.messages.len(), 0);
    }
}
