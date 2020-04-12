extern crate x20_grpc_rust as x20;

use std::sync::Arc;
use x20::X20Service;

#[derive(Clone)]
pub struct X20Client {
    client: Arc<x20::X20ServiceClient>,
    token: String,
}

impl X20Client {
    pub fn new(hostname: &str, port: u16, token: String) -> Self {
        Self {
            client: Arc::new(
                x20::X20ServiceClient::new_plain(hostname, port, Default::default()).unwrap(),
            ),
            token: token,
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

    pub fn publish_binary(&self, mut req: x20::PublishBinaryRequest) -> x20::PublishBinaryResponse {
        req.set_token(self.token.clone());
        self.client
            .publish_binary(std::default::Default::default(), req)
            .wait()
            .expect("rpc")
            .1
    }

    pub fn get_configs(&self, env: String) -> Vec<x20::Configuration> {
        let mut req = x20::GetConfigsRequest::new();
        req.set_environment(env);
        self.client
            .get_configs(std::default::Default::default(), req)
            .wait()
            .expect("rpc")
            .1
            .take_configs()
            .into_vec()
    }

    pub fn publish_config(&self, mut req: x20::PublishConfigRequest) -> x20::PublishConfigResponse {
        req.set_token(self.token.clone());
        self.client
            .publish_config(std::default::Default::default(), req)
            .wait()
            .expect("rpc")
            .1
    }
}
