use search_grpc_rust::*;
use search_lib::Searcher;

use auth_client::AuthServer;

use std::sync::Arc;

#[derive(Clone)]
pub struct SearchServiceHandler {
    searcher: Arc<Searcher>,
    auth: auth_client::AuthClient,
}

impl SearchServiceHandler {
    pub fn new(searcher: Arc<Searcher>, auth: auth_client::AuthClient) -> Self {
        Self {
            searcher: searcher,
            auth: auth,
        }
    }

    pub fn authenticate(&self, token: &str) -> bool {
        self.auth.authenticate(token.to_owned()).get_success()
    }
}

impl SearchService for SearchServiceHandler {
    fn search(
        &self,
        _: grpc::RequestOptions,
        req: SearchRequest,
    ) -> grpc::SingleResponse<SearchResponse> {
        if !self.authenticate(req.get_token()) {
            let mut response = SearchResponse::new();
            response.set_error(Error::AUTHENTICATION);
            return grpc::SingleResponse::completed(response);
        }

        let mut response = self.searcher.search(req.get_query());
        grpc::SingleResponse::completed(response)
    }
}
