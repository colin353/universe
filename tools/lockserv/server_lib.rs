use lockserv_bus::*;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use std::future::Future;
use std::pin::Pin;

const DEFAULT_TIMEOUT: u64 = 30_000_000;
const MAX_TIMEOUT: u64 = 60;

const XORSTATE: u64 = 0x2545F4914F6CDD1D;

pub fn get_timestamp_usec() -> u64 {
    let now = std::time::SystemTime::now();
    let since_epoch = now.duration_since(std::time::UNIX_EPOCH).unwrap();
    (since_epoch.as_secs() as u64) * 1_000_000 + (since_epoch.subsec_nanos() / 1000) as u64
}

struct Cell {
    content: Vec<u8>,
    generation: u64,
    timeout: u64,
    timestamp: u64,
    locked: bool,
}

impl Cell {
    pub fn new() -> Self {
        Self {
            content: Vec::new(),
            generation: 0,
            timeout: DEFAULT_TIMEOUT,
            timestamp: get_timestamp_usec(),
            locked: false,
        }
    }

    pub fn is_locked(&self) -> bool {
        self.locked && ((get_timestamp_usec() - self.timestamp) < self.timeout)
    }

    pub fn lock(&mut self) {
        self.locked = true;
        self.timestamp = get_timestamp_usec();
        self.generation += 1;
    }

    // The generation number is supposed to be "opaque" to consumers. It's obfuscated slightly by
    // this xorshift, so that client's can't accidentally guess the next generation number and
    // acquire somebody's lock.
    pub fn get_generation(&self) -> u64 {
        let mut x = self.generation;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        x.wrapping_mul(XORSTATE)
    }
}

#[derive(Clone)]
pub struct LockServiceHandler {
    cells: Arc<RwLock<HashMap<String, Cell>>>,
}

impl LockServiceHandler {
    pub fn new() -> Self {
        Self {
            cells: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn acquire(&self, req: AcquireRequest) -> AcquireResponse {
        let mut map = self.cells.write().unwrap();
        let cell = map.entry(req.path.to_owned()).or_insert(Cell::new());

        let mut response = AcquireResponse::new();
        response.content = cell.content.clone();

        // If the cell is already locked,
        if cell.is_locked() && req.generation != cell.get_generation() {
            return response;
        }

        if req.should_yield {
            cell.locked = false;
        } else {
            cell.lock();
        }
        if req.set_content {
            cell.content = req.content;
        }

        if req.timeout == 0 || req.timeout > MAX_TIMEOUT {
            cell.timeout = DEFAULT_TIMEOUT;
        } else {
            cell.timeout = req.timeout * 1_000_000;
        }

        response.generation = cell.get_generation();
        response.success = true;
        response
    }

    pub fn read(&self, req: ReadRequest) -> ReadResponse {
        let map = self.cells.read().unwrap();
        let cell = match map.get(&req.path) {
            Some(c) => c,
            None => {
                return ReadResponse::new();
            }
        };

        let mut response = ReadResponse::new();
        response.content = cell.content.clone();
        response.locked = cell.is_locked();
        response
    }
}

impl lockserv_bus::LockAsyncServiceHandler for LockServiceHandler {
    fn acquire(
        &self,
        req: AcquireRequest,
    ) -> Pin<Box<dyn Future<Output = Result<AcquireResponse, bus::BusRpcError>> + Send>> {
        let _self = self.clone();
        Box::pin(async move { Ok(_self.acquire(req)) })
    }

    fn read(
        &self,
        req: ReadRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ReadResponse, bus::BusRpcError>> + Send>> {
        let _self = self.clone();
        Box::pin(async move { Ok(_self.read(req)) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lock() {
        let ls = crate::LockServiceHandler::new();
        let mut req = AcquireRequest::new();
        req.path = "/my_lock".to_string();
        req.content = vec![1, 2, 3, 4];
        req.set_content = true;
        let response = ls.acquire(req);

        assert_eq!(response.success, true);
        assert_eq!(response.generation, 5180492295206395165);
        assert_eq!(response.content, &[]);

        // If we try to lock it again, it should fail
        let mut req = AcquireRequest::new();
        req.path = "/my_lock".to_string();
        let response = ls.acquire(req);

        assert_eq!(response.success, false);
        assert_eq!(response.content, &[1, 2, 3, 4]);

        // If we re-acquire the lock using the correct generation, should work
        let mut req = AcquireRequest::new();
        req.path = "/my_lock".to_string();
        req.generation = 5180492295206395165;
        let response = ls.acquire(req);

        assert_eq!(response.success, true);
        assert_eq!(response.content, &[1, 2, 3, 4]);
        assert_eq!(response.generation, 10360984590412790330);

        // Now we'll yield the lock
        let mut req = AcquireRequest::new();
        req.path = "/my_lock".to_string();
        req.generation = 10360984590412790330;
        req.should_yield = true;
        let response = ls.acquire(req);

        assert_eq!(response.success, true);

        // It should be unlocked, so a request w/o generation should work
        let mut req = AcquireRequest::new();
        req.path = "/my_lock".to_string();
        let response = ls.acquire(req);

        assert_eq!(response.success, true);
    }

    #[tokio::test]
    async fn test_lock_expiry() {
        let ls = crate::LockServiceHandler::new();
        let mut req = AcquireRequest::new();
        req.path = "/my_lock".to_string();
        req.timeout = 1;
        let response = ls.acquire(req);

        assert_eq!(response.success, true);

        // Attempting to re-acquire the lock should fail
        let mut req = AcquireRequest::new();
        req.path = "/my_lock".to_string();
        req.timeout = 1;
        let response = ls.acquire(req);

        assert_eq!(response.success, false);

        // Let's read the lock and check that it's locked
        let mut req = ReadRequest::new();
        req.path = "/my_lock".to_string();
        let response = ls.read(req);

        assert_eq!(response.locked, true);

        // Wait for the lock to expire
        tokio::time::delay_for(std::time::Duration::from_secs(2)).await;

        // Let's read the lock and check that it expired
        let mut req = ReadRequest::new();
        req.path = "/my_lock".to_string();
        let response = ls.read(req);

        assert_eq!(response.locked, false);

        // Now the lock should be undefended, so we can acquire it
        let mut req = AcquireRequest::new();
        req.path = "/my_lock".to_string();
        req.timeout = 1;
        let response = ls.acquire(req);

        assert_eq!(response.success, true);
        assert_eq!(response.generation, 10360984590412790330);
    }
}
