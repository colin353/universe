extern crate x20_grpc_rust as x20;

use grpc::{ClientStub, ClientStubExt};
use std::sync::Arc;

fn wait<T: Send + Sync>(resp: grpc::SingleResponse<T>) -> Result<T, grpc::Error> {
    futures::executor::block_on(resp.join_metadata_result()).map(|r| r.1)
}

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

    pub fn new_tls(hostname: &str, port: u16, token: String) -> Self {
        let grpc_client = grpc_tls::make_tls_client(hostname, port);
        Self {
            client: Arc::new(x20::X20ServiceClient::with_client(Arc::new(grpc_client))),
            token: token,
        }
    }

    pub fn get_binaries(&self) -> Result<Vec<x20::Binary>, x20::Error> {
        match wait(self.client.get_binaries(
            std::default::Default::default(),
            x20::GetBinariesRequest::new(),
        )) {
            Ok(mut x) => Ok(x.take_binaries().into_vec()),
            Err(_) => Err(x20::Error::NETWORK),
        }
    }

    pub fn publish_binary(&self, mut req: x20::PublishBinaryRequest) -> x20::PublishBinaryResponse {
        req.set_token(self.token.clone());
        wait(
            self.client
                .publish_binary(std::default::Default::default(), req),
        )
        .unwrap()
    }

    pub fn get_configs(&self, env: String) -> Result<Vec<x20::Configuration>, x20::Error> {
        let mut req = x20::GetConfigsRequest::new();
        req.set_environment(env);
        match wait(
            self.client
                .get_configs(std::default::Default::default(), req),
        ) {
            Ok(mut x) => Ok(x.take_configs().into_vec()),
            Err(_) => Err(x20::Error::NETWORK),
        }
    }

    pub fn publish_config(&self, mut req: x20::PublishConfigRequest) -> x20::PublishConfigResponse {
        req.set_token(self.token.clone());
        wait(
            self.client
                .publish_config(std::default::Default::default(), req),
        )
        .expect("rpc")
    }
}
