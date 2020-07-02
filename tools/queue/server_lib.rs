use largetable_client::LargeTableClient;
use queue_client::message_to_lockserv_path;
use queue_grpc_rust::*;

use std::collections::{HashMap, HashSet};
use std::iter::FromIterator;
use std::sync::{Arc, RwLock};

use futures::stream::Stream;
use futures::sync::mpsc;
use futures::sync::mpsc::UnboundedSender;

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
    s == Status::CREATED || s == Status::RETRY || s == Status::CONTINUE
}

fn is_bumpable_status(s: Status) -> bool {
    s == Status::STARTED || s == Status::BLOCKED
}

pub fn is_complete_status(s: Status) -> bool {
    s == Status::SUCCESS || s == Status::FAILURE
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
pub struct QueueServiceHandler<C: LargeTableClient + Clone + Send + Sync + 'static> {
    database: C,
    lockserv_client: Option<lockserv_client::LockservClient>,
    queues: Arc<RwLock<HashSet<String>>>,
    router: Arc<MessageRouter>,
    base_url: String,
}

impl<C: LargeTableClient + Clone + Send + Sync + 'static> QueueServiceHandler<C> {
    pub fn new(
        database: C,
        lockserv_client: lockserv_client::LockservClient,
        base_url: String,
    ) -> Self {
        // Set up compaction policy
        let mut policy = largetable_client::CompactionPolicy::new();
        policy.set_row(QUEUE.to_owned());
        policy.set_scope(String::new());
        policy.set_history(1);
        database.set_compaction_policy(policy);

        let iter = largetable_client::LargeTableScopedIterator::<Message, C>::new(
            &database,
            QUEUES.to_string(),
            String::new(),
            String::new(),
            String::new(),
            0,
        )
        .map(|(k, _)| k);

        let queues = HashSet::from_iter(iter);

        Self {
            database,
            queues: Arc::new(RwLock::new(queues)),
            lockserv_client: Some(lockserv_client),
            router: Arc::new(MessageRouter::new()),
            base_url,
        }
    }

    pub fn new_fake(database: C) -> Self {
        Self {
            database,
            queues: Arc::new(RwLock::new(HashSet::new())),
            lockserv_client: None,
            router: Arc::new(MessageRouter::new()),
            base_url: String::new(),
        }
    }

    pub fn maybe_create_queue(&self, queue: &str) {
        if self.queues.read().unwrap().contains(queue) {
            return;
        }

        // Update the cache
        {
            let mut queues = self.queues.write().unwrap();
            queues.insert(queue.to_string());
        }

        // Create the queue
        self.database.write(QUEUES, queue, 0, Vec::new());
    }

    pub fn enqueue(&self, mut req: EnqueueRequest) -> EnqueueResponse {
        self.maybe_create_queue(req.get_queue());

        // First, reserve an ID for this task
        let id = self.database.reserve_id(MESSAGE_IDS, req.get_queue());

        let mut message = req.take_msg();
        message.set_id(id);
        message.set_status(Status::CREATED);
        message.set_queue(req.get_queue().to_owned());
        message.set_enqueued_time(get_timestamp_usec());

        self.update(message.clone());
        self.router.put(req.get_queue(), &message);

        let mut response = EnqueueResponse::new();
        response.set_id(id);
        response
    }

    fn get_queue_limit(&self, queue: &str) -> u64 {
        // If this is the oldest in-progress task, then bump up
        // the queue window.
        match self
            .database
            .read_proto::<QueueWindowLimit>(&get_queue_window_rowname(), queue, 0)
        {
            Some(l) => l.get_limit(),
            None => 0,
        }
    }

    pub fn update(&self, msg: Message) -> UpdateResponse {
        self.database.write_proto(
            &get_queue_rowname(msg.get_queue()),
            &get_colname(msg.get_id()),
            0,
            &msg,
        );

        UpdateResponse::new()
    }

    pub fn read(&self, queue: &str, id: u64) -> Option<Message> {
        self.database
            .read_proto(&get_queue_rowname(queue), &get_colname(id), 0)
            .map(|mut m: Message| {
                m.set_info_url(format!("{}queue/{}/{}", self.base_url, queue, id));
                m
            })
    }

    pub fn consume(&self, req: ConsumeRequest) -> ConsumeResponse {
        let limit = self.get_queue_limit(req.get_queue());
        let mut iter = largetable_client::LargeTableScopedIterator::<Message, C>::new(
            &self.database,
            get_queue_rowname(req.get_queue()),
            String::new(),
            get_colname(limit),
            String::new(),
            0,
        )
        .map(|(_, m)| m);

        let mut limit_bump = 0;
        let mut all_complete = true;
        let mut response = ConsumeResponse::new();
        for msg in iter {
            if all_complete && is_complete_status(msg.get_status()) {
                limit_bump += 1;
            } else {
                all_complete = false;
            }

            if is_consumable_status(msg.get_status()) {
                response.mut_messages().push(msg);
                if response.get_messages().len() > 10 {
                    break;
                }
            }
        }

        // Possibly update the bump limit
        if limit_bump > 0 {
            let mut new_limit = QueueWindowLimit::new();
            new_limit.set_limit(limit + limit_bump);
            self.database
                .write_proto(&get_queue_window_rowname(), req.get_queue(), 0, &new_limit);
        }

        response
    }

    // This method watches for changes that were started but timed out, and
    // puts them back onto the queue.
    pub fn bump(&self) {
        let queues: Vec<_> = largetable_client::LargeTableScopedIterator::<Message, C>::new(
            &self.database,
            QUEUES.to_string(),
            String::new(),
            String::new(),
            String::new(),
            0,
        )
        .map(|(k, _)| k)
        .collect();

        for queue in queues {
            self.bump_queue(queue);
        }
    }

    pub fn bump_queue(&self, queue: String) {
        let limit = self.get_queue_limit(&queue);
        let eligible_messages: Vec<_> =
            largetable_client::LargeTableScopedIterator::<Message, C>::new(
                &self.database,
                get_queue_rowname(&queue),
                String::new(),
                get_colname(limit),
                String::new(),
                0,
            )
            .map(|(_, m)| m)
            .filter(|m| is_bumpable_status(m.get_status()))
            .collect();

        for message in eligible_messages {
            let lock = match self
                .lockserv_client
                .as_ref()
                .unwrap()
                .acquire(message_to_lockserv_path(&message))
            {
                Ok(l) => l,
                Err(_) => continue,
            };

            // Reload the message from the database now that we got the lock
            let mut message = match self.read(&queue, message.get_id()) {
                Some(m) => m,
                None => continue,
            };

            if message.get_status() == Status::STARTED {
                println!(
                    "message {} started but failed, retrying...",
                    message.get_id()
                );
                // We should never be able to acquire a lock on a started process, so
                // it must have failed.
                message.set_failures(message.get_failures() + 1);

                if message.get_failures() >= MAX_RETRIES {
                    message.set_status(Status::FAILURE);
                    message.set_reason(String::from("reached max retries"));
                } else {
                    message.set_status(Status::RETRY);
                }

                self.update(message.clone());

                if is_consumable_status(message.get_status()) {
                    self.router.put(&queue, &message);
                }
            } else if message.get_status() == Status::BLOCKED {
                // Check if blocked messages are unblocked yet, and return them to
                // the queue with CONTINUE status if they're unblocked.
                let mut blocked = false;
                let mut error = false;
                for blocker in message.get_blocked_by() {
                    let m = match self.read(blocker.get_queue(), blocker.get_id()) {
                        Some(m) => m,
                        None => {
                            error = true;
                            break;
                        }
                    };
                    if !is_complete_status(m.get_status()) {
                        blocked = true;
                        break;
                    }
                }

                if error {
                    message.set_status(Status::FAILURE);
                    message.set_reason(String::from("blocked by unknown message!"));
                    self.update(message);
                } else if !blocked {
                    message.set_status(Status::CONTINUE);
                    self.update(message.clone());
                    self.router.put(&queue, &message);
                }
            }

            self.lockserv_client.as_ref().unwrap().yield_lock(lock);
        }
    }
}

impl<C: LargeTableClient + Clone + Send + Sync + 'static> queue_grpc_rust::QueueService
    for QueueServiceHandler<C>
{
    fn enqueue(
        &self,
        _m: grpc::RequestOptions,
        req: EnqueueRequest,
    ) -> grpc::SingleResponse<EnqueueResponse> {
        grpc::SingleResponse::completed(self.enqueue(req))
    }

    fn update(
        &self,
        _m: grpc::RequestOptions,
        req: Message,
    ) -> grpc::SingleResponse<UpdateResponse> {
        grpc::SingleResponse::completed(self.update(req))
    }

    fn consume(
        &self,
        _m: grpc::RequestOptions,
        req: ConsumeRequest,
    ) -> grpc::SingleResponse<ConsumeResponse> {
        grpc::SingleResponse::completed(self.consume(req))
    }

    fn read(
        &self,
        _m: grpc::RequestOptions,
        req: ReadRequest,
    ) -> grpc::SingleResponse<ReadResponse> {
        let mut response = ReadResponse::new();
        let maybe_message = self.read(&req.get_queue(), req.get_id());
        match maybe_message {
            Some(m) => {
                response.set_found(true);
                *response.mut_msg() = m;
            }
            None => (),
        };

        grpc::SingleResponse::completed(response)
    }

    fn consume_stream(
        &self,
        _m: grpc::RequestOptions,
        req: ConsumeRequest,
    ) -> grpc::StreamingResponse<ConsumeResponse> {
        let (tx, rx) = mpsc::unbounded();
        self.router.subscribe(req.get_queue().to_owned(), tx);

        let initial_response = self.consume(req);
        let stream: Box<
            dyn futures::Stream<Item = ConsumeResponse, Error = grpc::Error> + Send + 'static,
        > = if initial_response.get_messages().len() > 0 {
            Box::new(
                futures::stream::iter_ok(vec![initial_response])
                    .chain(rx.map(|m| {
                        let mut resp = ConsumeResponse::new();
                        resp.mut_messages().push(m);
                        resp
                    }))
                    .map_err(|_| grpc::Error::Other("stream error")),
            )
        } else {
            Box::new(
                rx.map(|m| {
                    let mut resp = ConsumeResponse::new();
                    resp.mut_messages().push(m);
                    resp
                })
                .map_err(|_| grpc::Error::Other("stream error")),
            )
        };

        grpc::StreamingResponse::no_metadata(stream.map_err(|_| grpc::Error::Other("stream error")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate largetable_test;

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
        assert_eq!(response.get_messages().len(), 1);

        // Now let's update it to be in progress
        let mut msg = &mut response.mut_messages()[0];
        msg.set_status(Status::STARTED);
        q.update(msg.clone());

        // There shouldn't be any more messages available
        let mut req = ConsumeRequest::new();
        req.set_queue(String::from("test"));
        let response = q.consume(req);
        assert_eq!(response.get_messages().len(), 0);
    }
}
