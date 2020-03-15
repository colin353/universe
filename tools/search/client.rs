use search_grpc_rust::SearchService;
use std::sync::Arc;

pub struct SearchClient {
    client: Arc<search_grpc_rust::SearchServiceClient>,
}

impl SearchClient {
    pub fn new(hostname: &str, port: u16) -> Self {
        SearchClient {
            client: Arc::new(
                search_grpc_rust::SearchServiceClient::new_plain(
                    hostname,
                    port,
                    Default::default(),
                )
                .unwrap(),
            ),
        }
    }

    pub fn search(&self, req: search_grpc_rust::SearchRequest) -> search_grpc_rust::SearchResponse {
        self.client
            .search(std::default::Default::default(), req)
            .wait()
            .expect("rpc")
            .1
    }
}
