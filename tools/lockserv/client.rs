use grpc::ClientStubExt;
use lockserv_grpc_rust::*;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Locked,
    Network,
}

#[derive(Clone)]
pub struct LockservClient {
    client: Arc<LockServiceClient>,
    locks: Arc<Mutex<HashMap<String, Lock>>>,
}

#[derive(Debug, Clone)]
pub struct Lock {
    path: String,
    generation: u64,
    content: Vec<u8>,
}

impl Lock {
    pub fn new(path: String, generation: u64, content: Vec<u8>) -> Self {
        Self {
            path,
            generation,
            content,
        }
    }
}

impl LockservClient {
    pub fn new(hostname: &str, port: u16) -> Self {
        Self {
            client: Arc::new(
                LockServiceClient::new_plain(hostname, port, Default::default()).unwrap(),
            ),
            locks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn acquire(&self, path: String) -> Result<Lock, Error> {
        let mut req = AcquireRequest::new();
        req.set_path(path.clone());
        let mut response = match self.client.acquire(Default::default(), req).wait() {
            Ok(r) => r.1,
            Err(_) => return Err(Error::Network),
        };

        if response.get_success() {
            return Ok(Lock::new(
                path,
                response.get_generation(),
                response.take_content(),
            ));
        }

        Err(Error::Locked)
    }

    pub fn reacquire(&self, lock: Lock) -> Result<Lock, Error> {
        let mut req = AcquireRequest::new();
        req.set_path(lock.path.clone());
        req.set_generation(lock.generation);

        let mut response = match self.client.acquire(Default::default(), req).wait() {
            Ok(r) => r.1,
            Err(_) => return Err(Error::Network),
        };

        if response.get_success() {
            return Ok(Lock::new(
                lock.path,
                response.get_generation(),
                response.take_content(),
            ));
        }

        Err(Error::Locked)
    }

    pub fn yield_lock(&self, lock: Lock) -> Result<(), Error> {
        let mut req = AcquireRequest::new();
        req.set_generation(lock.generation);
        req.set_path(lock.path);
        req.set_should_yield(true);

        match self.client.acquire(Default::default(), req).wait() {
            Ok(_) => Ok(()),
            Err(_) => Err(Error::Network),
        }
    }

    pub fn write<T: protobuf::Message>(&self, lock: Lock, message: T) -> Result<Lock, Error> {
        let mut req = AcquireRequest::new();
        req.set_path(lock.path.clone());
        req.set_generation(lock.generation);
        req.set_set_content(true);

        let mut bytes = Vec::new();
        message.write_to_vec(&mut bytes).unwrap();
        req.set_content(bytes);

        let mut response = match self.client.acquire(Default::default(), req).wait() {
            Ok(r) => r.1,
            Err(_) => return Err(Error::Network),
        };

        if response.get_success() {
            return Ok(Lock::new(
                lock.path,
                response.get_generation(),
                response.take_content(),
            ));
        }

        Err(Error::Locked)
    }

    pub fn read<T: protobuf::Message>(&self, path: String) -> Result<(T, bool), Error> {
        let mut req = ReadRequest::new();
        req.set_path(path);

        let response = match self.client.read(Default::default(), req).wait() {
            Ok(r) => r.1,
            Err(_) => return Err(Error::Network),
        };
        let mut message = T::new();
        message.merge_from_bytes(response.get_content()).unwrap();

        Ok((message, response.get_locked()))
    }

    pub fn put_lock(&self, lock: Lock) {
        self.locks.lock().unwrap().insert(lock.path.clone(), lock);
    }

    pub fn take_lock(&self, path: &str) -> Option<Lock> {
        self.locks.lock().unwrap().remove(path)
    }

    // Runs forever, defending the locks added to the lock mutex
    pub fn defend(&self) {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(15));

            let mut to_remove = Vec::new();

            {
                let mut locks = self.locks.lock().unwrap();
                for lock in locks.values_mut() {
                    let l = lock.clone();
                    *lock = match self.reacquire(l) {
                        Ok(l) => l,
                        Err(_) => {
                            to_remove.push(lock.path.clone());
                            continue;
                        }
                    };
                }
            }
        }
    }
}
