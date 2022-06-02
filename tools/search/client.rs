use grpc::ClientStub;
use grpc::ClientStubExt;

use search_grpc_rust::Error;
use std::sync::Arc;

pub struct SearchClient {
    client: Arc<search_grpc_rust::SearchServiceClient>,
    token: String,
}

fn wait<T: Send + Sync>(resp: grpc::SingleResponse<T>) -> Result<T, grpc::Error> {
    futures::executor::block_on(resp.join_metadata_result()).map(|r| r.1)
}

impl SearchClient {
    pub fn new(hostname: &str, port: u16, token: String) -> Self {
        SearchClient {
            client: Arc::new(
                search_grpc_rust::SearchServiceClient::new_plain(
                    hostname,
                    port,
                    Default::default(),
                )
                .unwrap(),
            ),
            token: token,
        }
    }

    pub fn new_tls(hostname: &str, port: u16, token: String) -> Self {
        let grpc_client = grpc_tls::make_tls_client(hostname, port);
        SearchClient {
            client: Arc::new(search_grpc_rust::SearchServiceClient::with_client(
                Arc::new(grpc_client),
            )),
            token: token,
        }
    }

    pub fn search(
        &self,
        mut req: search_grpc_rust::SearchRequest,
    ) -> search_grpc_rust::SearchResponse {
        req.set_token(self.token.clone());
        let result = wait(self.client.search(std::default::Default::default(), req)).expect("rpc");
        if result.get_error() != Error::NONE {
            panic!("search error: {:?}", result.get_error());
        }
        result
    }
}
