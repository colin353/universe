use grpc::ClientStubExt;
use lockserv_grpc_rust::*;

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub enum Error {
    Locked,
    Network,
}

#[derive(Clone)]
pub struct LockservClient {
    client: Arc<LockServiceClient>,
    map: Arc<RwLock<HashMap<String, u64>>>,
}

impl LockservClient {
    pub fn new(hostname: &str, port: u16) -> Self {
        Self {
            client: Arc::new(
                LockServiceClient::new_plain(hostname, port, Default::default()).unwrap(),
            ),
            map: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn acquire(&self, path: String) -> Result<AcquireResponse, Error> {
        let mut req = AcquireRequest::new();
        req.set_path(path.clone());
        let response = match self.client.acquire(Default::default(), req).wait() {
            Ok(r) => r.1,
            Err(_) => return Err(Error::Network),
        };

        if response.get_success() {
            let mut m = self.map.write().unwrap();
            let g = m.entry(path).or_insert(0);
            *g = response.get_generation();
            return Ok(response);
        }

        Err(Error::Locked)
    }

    pub fn reacquire(&self, path: String) -> Result<AcquireResponse, Error> {
        let mut req = AcquireRequest::new();
        req.set_path(path.clone());

        if let Some(x) = self.map.read().unwrap().get(req.get_path()) {
            req.set_generation(*x);
        }

        let response = match self.client.acquire(Default::default(), req).wait() {
            Ok(r) => r.1,
            Err(_) => return Err(Error::Network),
        };

        if response.get_success() {
            let mut m = self.map.write().unwrap();
            let g = m.entry(path).or_insert(0);
            *g = response.get_generation();
            return Ok(response);
        }

        Err(Error::Locked)
    }

    pub fn yield_lock(&self, path: String) {
        let mut req = AcquireRequest::new();
        req.set_path(path.clone());
        req.set_should_yield(true);

        if let Some(x) = self.map.read().unwrap().get(req.get_path()) {
            req.set_generation(*x);
        }

        let response = self
            .client
            .acquire(Default::default(), req)
            .wait()
            .unwrap()
            .1;

        if response.get_success() {
            self.map.write().unwrap().remove(&path);
        }
    }

    pub fn write<T: protobuf::Message>(
        &self,
        path: String,
        message: T,
    ) -> Result<AcquireResponse, Error> {
        let mut req = AcquireRequest::new();
        req.set_path(path.clone());
        req.set_set_content(true);

        let mut bytes = Vec::new();
        message.write_to_vec(&mut bytes).unwrap();
        req.set_content(bytes);

        if let Some(x) = self.map.read().unwrap().get(req.get_path()) {
            req.set_generation(*x);
        }

        let response = match self.client.acquire(Default::default(), req).wait() {
            Ok(r) => r.1,
            Err(_) => return Err(Error::Network),
        };

        if response.get_success() {
            let mut m = self.map.write().unwrap();
            let g = m.entry(path).or_insert(0);
            *g = response.get_generation();

            return Ok(response);
        }

        Err(Error::Locked)
    }

    pub fn read<T: protobuf::Message>(&self, path: String) -> (T, bool) {
        let mut req = ReadRequest::new();
        req.set_path(path);

        let response = self.client.read(Default::default(), req).wait().unwrap().1;
        let mut message = T::new();
        message.merge_from_bytes(response.get_content()).unwrap();

        (message, response.get_locked())
    }
}
