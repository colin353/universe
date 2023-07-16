use lockserv_bus::*;

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Locked,
    Network,
}

#[derive(Clone)]
pub struct LockservClient {
    client: Arc<LockAsyncClient>,
    locks: Arc<Mutex<HashMap<String, Lock>>>,
}

#[derive(Debug, Clone)]
pub struct Lock {
    path: String,
    generation: u64,
}

impl Lock {
    pub fn new(path: String, generation: u64) -> Self {
        Self { path, generation }
    }
}

impl LockservClient {
    pub fn new_metal(service_name: &str) -> Self {
        let connector = Arc::new(bus_rpc::MetalAsyncClient::new(service_name));
        Self {
            client: Arc::new(LockAsyncClient::new(connector)),
            locks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn acquire(
        &self,
        path: String,
    ) -> Pin<Box<dyn Future<Output = Result<Lock, Error>> + Send>> {
        let _self = self.clone();
        Box::pin(async move {
            let mut req = AcquireRequest::new();
            req.path = path.clone();
            let response = match _self.client.acquire(req).await {
                Ok(r) => r,
                Err(_) => return Err(Error::Network),
            };

            if response.success {
                return Ok(Lock::new(path, response.generation));
            }

            Err(Error::Locked)
        })
    }

    pub fn reacquire(
        &self,
        lock: Lock,
    ) -> Pin<Box<dyn Future<Output = Result<Lock, Error>> + Send>> {
        let _self = self.clone();
        Box::pin(async move {
            let mut req = AcquireRequest::new();
            req.path = lock.path.clone();
            req.generation = lock.generation;

            let response = match _self.client.acquire(req).await {
                Ok(r) => r,
                Err(_) => return Err(Error::Network),
            };

            if response.success {
                return Ok(Lock::new(lock.path, response.generation));
            }

            Err(Error::Locked)
        })
    }

    pub fn yield_lock(
        &self,
        lock: Lock,
    ) -> Pin<Box<dyn Future<Output = Result<(), Error>> + Send>> {
        let _self = self.clone();
        Box::pin(async move {
            let mut req = AcquireRequest::new();
            req.generation = lock.generation;
            req.path = lock.path;
            req.should_yield = true;

            match _self.client.acquire(req).await {
                Ok(_) => Ok(()),
                Err(_) => Err(Error::Network),
            }
        })
    }

    pub fn write<T: bus::Serialize>(
        &self,
        lock: Lock,
        message: T,
    ) -> Pin<Box<dyn Future<Output = Result<Lock, Error>> + Send>> {
        let mut req = AcquireRequest::new();
        req.path = lock.path.clone();
        req.generation = lock.generation;
        req.set_content = true;

        let mut bytes = Vec::new();
        message.encode(&mut bytes).unwrap();
        req.content = bytes;

        let _self = self.clone();

        Box::pin(async move {
            let response = match _self.client.acquire(req).await {
                Ok(r) => r,
                Err(_) => return Err(Error::Network),
            };

            if response.success {
                return Ok(Lock::new(lock.path, response.generation));
            }

            Err(Error::Locked)
        })
    }

    pub fn read<T: bus::DeserializeOwned>(
        &self,
        path: String,
    ) -> Pin<Box<dyn Future<Output = Result<(T, bool), Error>> + Send>> {
        let _self = self.clone();
        Box::pin(async move {
            let mut req = ReadRequest::new();
            req.path = path;

            let response = match _self.client.read(req).await {
                Ok(r) => r,
                Err(_) => return Err(Error::Network),
            };
            let message = T::decode_owned(&response.content).map_err(|_| Error::Network)?;
            Ok((message, response.locked))
        })
    }

    pub fn put_lock(&self, lock: Lock) {
        self.locks.lock().unwrap().insert(lock.path.clone(), lock);
    }

    pub fn take_lock(&self, path: &str) -> Option<Lock> {
        self.locks.lock().unwrap().remove(path)
    }

    // Runs forever, defending the locks added to the lock mutex
    pub fn defend(&self) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        let _self = self.clone();
        Box::pin(async move {
            loop {
                tokio::time::delay_for(std::time::Duration::from_secs(15)).await;

                let to_acquire = _self.locks.lock().unwrap().clone();
                for (key, lock) in to_acquire {
                    match _self.reacquire(lock).await {
                        Ok(l) => {
                            _self.locks.lock().unwrap().insert(key, l);
                        }
                        Err(_) => {
                            _self.locks.lock().unwrap().remove(&key);
                        }
                    }
                }
            }
        })
    }
}
