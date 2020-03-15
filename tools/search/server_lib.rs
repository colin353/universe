use search_grpc_rust::*;
use search_lib::Searcher;

use std::sync::Arc;

#[derive(Clone)]
pub struct SearchServiceHandler {
    searcher: Arc<Searcher>,
}

impl SearchServiceHandler {
    pub fn new(searcher: Arc<Searcher>) -> Self {
        Self { searcher: searcher }
    }
}

impl SearchService for SearchServiceHandler {
    fn search(
        &self,
        _: grpc::RequestOptions,
        req: SearchRequest,
    ) -> grpc::SingleResponse<SearchResponse> {
        let mut response = SearchResponse::new();
        for result in self.searcher.search(req.get_query()) {
            response.mut_candidates().push(result);
        }
        grpc::SingleResponse::completed(response)
    }
}
