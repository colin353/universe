use lockserv_client::*;
use lockserv_grpc_rust::DataMessage;

//#[test]
fn test_connect() {
    // Acquire the initial lock
    let c1 = LockservClient::new("127.0.0.1", 5555);
    let result = c1.acquire(String::from("/my_test"));
    assert!(result.is_ok());
    let generation = result.unwrap().get_generation();

    // Attempt to acquire from another connection
    let c2 = LockservClient::new("127.0.0.1", 5555);
    let result = c2.acquire(String::from("/my_test"));
    assert_eq!(result, Err(Error::Locked));

    // Re-acquire the lock from first conn, should work
    let result = c1.reacquire(String::from("/my_test"));
    assert!(result.is_ok());
    let generation = result.unwrap().get_generation();

    // Yield the lock
    c1.yield_lock(String::from("/my_test"));

    // Try connecting from other connection, should work
    let result = c2.acquire(String::from("/my_test"));
    assert!(result.is_ok());

    // Yield that lock to clean up the state
    c2.yield_lock(String::from("/my_test"));
}

//#[test]
fn test_read_write() {
    let c = LockservClient::new("127.0.0.1", 5555);
    let mut msg = DataMessage::new();
    msg.set_data(String::from("hello world"));
    c.write(String::from("/data"), msg);

    let (out, locked): (DataMessage, bool) = c.read(String::from("/data"));
    assert_eq!(out.get_data(), "hello world");
    assert_eq!(locked, true);

    // Yield that lock to clean up the state
    c.yield_lock(String::from("/data"));
}
