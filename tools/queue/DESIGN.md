# Queue design

The idea is: messages are enqueued into a queue keyed by a string.  Then,
consumers poll the queue (or use a streaming gRPC request) to get new messages
posted to the queue.

In order to claim a message, the consumer must claim an advisory lock on the
message using lockserv. It must also defend that lock against timeouts while
processing the message.

Once done processing the message, if it still holds the advisory lock, it can
update the message status to DONE.

## Blocking

Messages can be blocked by other messages. To do this, the consumer acquires a
lock on the message, enqueues messages which block this message, then updates
its status to BLOCKED and specifies a `blocked_by` parameter with the IDs of
the enqueued messages.

BLOCKED messages will not be shown to consumers until their submessages are
finished.  Once they're finished, they'll appear again to a consumer, which can
inspect the finished messages state and pick up where they left off.

## When polling the queue, what do we return?

When the queue is polled, we will only return messages with eligible statues,
i.e. CREATED or RETRY, or CONTINUE. Another background process will check messages for expired
locks, try to acquire that lock, and mark the messages as RETRY so they show up in
the queue again.

The same is done for BLOCKED messages, but the task will check the dependent tasks
to see whether those are blocked.
