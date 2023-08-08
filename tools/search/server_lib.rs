use search_grpc_rust::*;
use search_lib::Searcher;

use auth_client::AuthServer;

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
        _: grpc::ServerHandlerContext,
        req: grpc::ServerRequestSingle<SearchRequest>,
        resp: grpc::ServerResponseUnarySink<SearchResponse>,
    ) -> grpc::Result<()> {
        let mut response = self.searcher.search(req.message.get_query());
        resp.finish(response)
    }

    fn suggest(
        &self,
        _: grpc::ServerHandlerContext,
        req: grpc::ServerRequestSingle<SuggestRequest>,
        resp: grpc::ServerResponseUnarySink<SuggestResponse>,
    ) -> grpc::Result<()> {
        let mut response = self.searcher.suggest(req.message.get_prefix());
        resp.finish(response)
    }
}
