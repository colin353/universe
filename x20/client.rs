extern crate x20_grpc_rust as x20;

use std::sync::Arc;
use x20::X20Service;

#[derive(Clone)]
pub struct X20Client {
    client: Arc<x20::X20ServiceClient>,
}

impl X20Client {
    pub fn new(hostname: &str, port: u16) -> Self {
        Self {
            client: Arc::new(
                x20::X20ServiceClient::new_plain(hostname, port, Default::default()).unwrap(),
            ),
        }
    }

    pub fn get_binaries(&self) -> Vec<x20::Binary> {
        self.client
            .get_binaries(
                std::default::Default::default(),
                x20::GetBinariesRequest::new(),
            )
            .wait()
            .expect("rpc")
            .1
            .take_binaries()
            .into_vec()
    }

    pub fn publish_binary(&self, req: x20::PublishBinaryRequest) -> x20::PublishBinaryResponse {
        self.client
            .publish_binary(std::default::Default::default(), req)
            .wait()
            .expect("rpc")
            .1
    }
}
