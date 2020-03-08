use search_grpc_rust::*;
use search_lib::Searcher;

use std::sync::Arc;

#[derive(Clone)]
pub struct SearchServiceHandler {
    searcher: Arc<Searcher>,
}

impl SearchServiceHandler {
    pub fn new(searcher: Searcher) -> Self {
        Self {
            searcher: Arc::new(searcher),
        }
    }
}

impl SearchService for SearchServiceHandler {
    fn search(
        &self,
        _: grpc::RequestOptions,
        req: SearchRequest,
    ) -> grpc::SingleResponse<SearchResponse> {
        grpc::SingleResponse::completed(SearchResponse::new())
    }
}
