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
        _: grpc::ServerHandlerContext,
        req: grpc::ServerRequestSingle<SearchRequest>,
        resp: grpc::ServerResponseUnarySink<SearchResponse>,
    ) -> grpc::Result<()> {
        if !self.authenticate(req.message.get_token()) {
            let mut response = SearchResponse::new();
            response.set_error(Error::AUTHENTICATION);
            return resp.finish(response);
        }

        let mut response = self.searcher.search(req.message.get_query());
        resp.finish(response)
    }

    fn suggest(
        &self,
        _: grpc::ServerHandlerContext,
        req: grpc::ServerRequestSingle<SuggestRequest>,
        resp: grpc::ServerResponseUnarySink<SuggestResponse>,
    ) -> grpc::Result<()> {
        if !self.authenticate(req.message.get_token()) {
            let mut response = SuggestResponse::new();
            response.set_error(Error::AUTHENTICATION);
            return resp.finish(response);
        }

        let mut response = self.searcher.suggest(req.message.get_prefix());
        resp.finish(response)
    }
}
