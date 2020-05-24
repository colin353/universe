# Lockserv

Lockserv is an in-memory, non-persistent advisory locking service.
Clients can write small amounts of data into cells, which can be
locked. 

Clients hold a lock as long as they want, but must defend the
lock by re-acquiring it before the timeout.

When a lock is held, the client holds a special number called the
generation number. It's an opaque number that allows for renewal
of the lock. The generation number can also be passed from the
client to other services to pass ownership of the lock.

To modify the cell contents while the lock is held, or renew the lock, the
correct generation number must be provided. The generation numbers are just
incremented integers, but they're made more opaque by scrambling the contents
to make it less probable that clients would accidentally send valid generation
numbers.
