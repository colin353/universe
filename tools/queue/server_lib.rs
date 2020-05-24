use largetable_client::LargeTableClient;
use queue_grpc_rust::*;

const QUEUES: &'static str = "queues";

fn get_timestamp_usec() -> u64 {
    let now = std::time::SystemTime::now();
    let since_epoch = now.duration_since(std::time::UNIX_EPOCH).unwrap();
    (since_epoch.as_secs() as u64) * 1_000_000 + (since_epoch.subsec_nanos() / 1000) as u64
}

fn get_queue_rowname(queue: &str) -> String {
    format!("{}/{}", QUEUES, queue)
}

fn get_message_rowname() -> String {
    format!("{}/m", QUEUES)
}

fn get_queue_window_rowname(queue: &str) -> String {
    format!("{}/limit", QUEUES)
}

fn get_colname(id: u64) -> String {
    format!("{:016x}", id)
}

fn is_consumable_status(s: Status) -> bool {
    s == Status::CREATED || s == Status::RETRY || s == Status::CONTINUE
}

#[derive(Clone)]
pub struct QueueServiceHandler<C: LargeTableClient + Clone + Send + Sync + 'static> {
    database: C,
}

impl<C: LargeTableClient + Clone + Send + Sync + 'static> QueueServiceHandler<C> {
    pub fn new(database: C) -> Self {
        // Set up compaction policy
        let mut policy = largetable_client::CompactionPolicy::new();
        policy.set_row(QUEUES.to_owned());
        policy.set_scope(String::new());
        policy.set_history(1);
        database.set_compaction_policy(policy);

        Self { database: database }
    }

    pub fn enqueue(&self, mut req: EnqueueRequest) -> EnqueueResponse {
        // First, reserve an ID for this task
        let id = self.database.reserve_id(QUEUES, req.get_queue());

        let mut message = req.take_msg();
        message.set_id(id);
        message.set_status(Status::CREATED);
        message.set_queue(req.get_queue().to_owned());
        message.set_enqueued_time(get_timestamp_usec());

        // Write it into the queue
        self.database.write_proto(
            &get_queue_rowname(req.get_queue()),
            &get_colname(id),
            0,
            &message,
        );

        // Also write to the message index
        self.database
            .write_proto(&get_message_rowname(), &get_colname(id), 0, &message);

        EnqueueResponse::new()
    }

    fn get_queue_limit(&self, queue: &str) -> u64 {
        // If this is the oldest in-progress task, then bump up
        // the queue window.
        match self
            .database
            .read_proto::<QueueWindowLimit>(&get_queue_window_rowname(queue), "", 0)
        {
            Some(l) => l.get_limit(),
            None => 0,
        }
    }

    pub fn update(&self, msg: Message) -> UpdateResponse {
        // Update inside the active queue
        self.database.write_proto(
            &get_queue_rowname(msg.get_queue()),
            &get_colname(msg.get_id()),
            0,
            &msg,
        );

        // Also update id-indexed row
        self.database
            .write_proto(&get_message_rowname(), &get_colname(msg.get_id()), 0, &msg);

        if msg.get_status() == Status::SUCCESS || msg.get_status() == Status::FAILURE {
            let limit = self.get_queue_limit(msg.get_queue());
            if msg.get_id() == limit + 1 {
                let mut new_limit = QueueWindowLimit::new();
                new_limit.set_limit(msg.get_id());
                self.database.write_proto(
                    &get_queue_window_rowname(msg.get_queue()),
                    "",
                    0,
                    &new_limit,
                );
            }
        }

        UpdateResponse::new()
    }

    pub fn consume(&self, req: ConsumeRequest) -> ConsumeResponse {
        let limit = self.get_queue_limit(req.get_queue());

        let mut eligible_messages: Vec<_> =
            largetable_client::LargeTableScopedIterator::<Message, C>::new(
                &self.database,
                get_queue_rowname(req.get_queue()),
                String::new(),
                get_colname(limit),
                String::new(),
                0,
            )
            .map(|(_, m)| m)
            .filter(|m| is_consumable_status(m.get_status()))
            .take(5)
            .collect();

        let mut response = ConsumeResponse::new();
        if eligible_messages.len() > 0 {
            // TODO: maybe randomly choose one of the 5 oldest?
            response.set_msg(eligible_messages.swap_remove(0));
            response.set_message_available(true);
        }

        response
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
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate largetable_test;

    fn make_handler() -> QueueServiceHandler<largetable_test::LargeTableMockClient> {
        let database = largetable_test::LargeTableMockClient::new();
        QueueServiceHandler::new(database)
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
        assert_eq!(response.get_message_available(), true);

        // Now let's update it to be in progress
        let mut msg = response.take_msg();
        msg.set_status(Status::STARTED);
        q.update(msg);

        // There shouldn't be any more messages available
        let mut req = ConsumeRequest::new();
        req.set_queue(String::from("test"));
        let response = q.consume(req);
        assert_eq!(response.get_message_available(), false);
    }
}
