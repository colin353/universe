use search_grpc_rust::{Error, SearchService};
use std::sync::Arc;

pub struct SearchClient {
    client: Arc<search_grpc_rust::SearchServiceClient>,
    token: String,
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

    pub fn search(
        &self,
        mut req: search_grpc_rust::SearchRequest,
    ) -> search_grpc_rust::SearchResponse {
        req.set_token(self.token.clone());
        let result = self
            .client
            .search(std::default::Default::default(), req)
            .wait()
            .expect("rpc")
            .1;
        if result.get_error() != Error::NONE {
            panic!("search error: {:?}", result.get_error());
        }
        result
    }
}
