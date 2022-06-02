extern crate auth_client;
extern crate bugs_grpc_rust as bugs;
extern crate grpc;
extern crate largetable_client;

use auth_client::AuthServer;
use largetable_client::LargeTableClient;

const BUG_IDS: &'static str = "bug_ids";
const ALL_BUGS: &'static str = "all_bugs";

#[derive(Clone)]
pub struct BugServiceHandler<C: LargeTableClient> {
    database: C,
    auth: Option<auth_client::AuthClient>,
}

fn bug_rowname(s: bugs::BugStatus) -> String {
    format!("bugs::{:?}", s)
}

fn bug_colname(id: u64) -> String {
    id.to_string()
}

impl<C: LargeTableClient + Clone> BugServiceHandler<C> {
    pub fn new(db: C, auth: auth_client::AuthClient) -> Self {
        Self {
            database: db,
            auth: Some(auth),
        }
    }

    pub fn get_bugs(&self, req: &bugs::GetBugsRequest) -> bugs::GetBugsResponse {
        let bug_iter = largetable_client::LargeTableScopedIterator::new(
            &self.database,
            bug_rowname(req.get_status()),
            String::from(""),
            String::from(""),
            String::from(""),
            0,
        );
        let mut response = bugs::GetBugsResponse::new();
        for (_, bug) in bug_iter {
            response.mut_bugs().push(bug);
        }
        response
    }

    pub fn get_bug(&self, req: &bugs::Bug) -> bugs::Bug {
        match self
            .database
            .read_proto(ALL_BUGS, &bug_colname(req.get_id()), 0)
        {
            Some(b) => b,
            None => return bugs::Bug::new(),
        }
    }

    pub fn create_bug(&self, mut req: bugs::Bug) -> bugs::Bug {
        let id = self.database.reserve_id(BUG_IDS, "");
        req.set_id(id);
        let rowname = bug_rowname(req.get_status());
        let colname = bug_colname(req.get_id());
        self.database.write_proto(&rowname, &colname, 0, &req);
        self.database.write_proto(ALL_BUGS, &colname, 0, &req);
        req
    }

    pub fn update_bug(&self, req: bugs::Bug) -> bugs::Bug {
        let prior_bug = self.get_bug(&req);
        if prior_bug.get_id() == 0 {
            return bugs::Bug::new();
        }

        let rowname = bug_rowname(req.get_status());
        let colname = bug_colname(req.get_id());
        self.database.write_proto(&rowname, &colname, 0, &req);
        self.database.write_proto(ALL_BUGS, &colname, 0, &req);

        // If it was previously in a different queue, let's delete it from the old queue
        let rowname = bug_rowname(prior_bug.get_status());
        if prior_bug.get_status() != req.get_status() {
            self.database.delete(&rowname, &colname);
        }

        req
    }

    fn authenticate(&self, token: &str) -> bool {
        self.auth
            .as_ref()
            .unwrap()
            .authenticate(token.to_owned())
            .get_success()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate largetable_test;

    fn create_test_handler() -> BugServiceHandler<largetable_test::LargeTableMockClient> {
        let db = largetable_test::LargeTableMockClient::new();
        BugServiceHandler {
            database: db,
            auth: None,
        }
    }

    #[test]
    fn test_create() {
        let handler = create_test_handler();

        let mut req = bugs::Bug::new();
        req.set_title(String::from("my bug"));
        handler.create_bug(req);

        // Should be able to read that back
        let mut req = bugs::GetBugsRequest::new();
        req.set_status(bugs::BugStatus::WAITING);
        let response = handler.get_bugs(&req);

        assert_eq!(response.get_bugs().len(), 1);
        assert_eq!(response.get_bugs()[0].get_title(), "my bug");
        assert_eq!(response.get_bugs()[0].get_id(), 1);
    }

    #[test]
    fn test_update() {
        let handler = create_test_handler();

        let mut req = bugs::Bug::new();
        req.set_title(String::from("my bug"));
        let mut b = handler.create_bug(req.clone());

        b.set_status(bugs::BugStatus::IN_PROGRESS);
        handler.update_bug(b);

        // Should be removed from the waiting queue
        let mut req = bugs::GetBugsRequest::new();
        req.set_status(bugs::BugStatus::WAITING);
        let response = handler.get_bugs(&req);
        assert_eq!(response.get_bugs().len(), 0);

        // Should now be on the in progress queue
        let mut req = bugs::GetBugsRequest::new();
        req.set_status(bugs::BugStatus::IN_PROGRESS);
        let response = handler.get_bugs(&req);
        assert_eq!(response.get_bugs().len(), 1);
    }
}

impl<C: LargeTableClient + Clone> bugs::BugService for BugServiceHandler<C> {
    fn get_bugs(
        &self,
        _: grpc::ServerHandlerContext,
        mut req: grpc::ServerRequestSingle<bugs::GetBugsRequest>,
        resp: grpc::ServerResponseUnarySink<bugs::GetBugsResponse>,
    ) -> grpc::Result<()> {
        if !self.authenticate(req.message.get_token()) {
            let mut response = bugs::GetBugsResponse::new();
            response.set_error(bugs::Error::AUTHENTICATION);
            return resp.finish(response);
        }

        resp.finish(self.get_bugs(&req.message))
    }

    fn get_bug(
        &self,
        _: grpc::ServerHandlerContext,
        mut req: grpc::ServerRequestSingle<bugs::GetBugRequest>,
        resp: grpc::ServerResponseUnarySink<bugs::GetBugResponse>,
    ) -> grpc::Result<()> {
        if !self.authenticate(req.message.get_token()) {
            let mut response = bugs::GetBugResponse::new();
            response.set_error(bugs::Error::AUTHENTICATION);
            return resp.finish(response);
        }

        let mut response = bugs::GetBugResponse::new();
        *response.mut_bug() = self.get_bug(req.message.get_bug());
        if response.get_bug().get_id() > 0 {
            response.set_found(true);
        }
        resp.finish(response)
    }

    fn create_bug(
        &self,
        _: grpc::ServerHandlerContext,
        mut req: grpc::ServerRequestSingle<bugs::CreateBugRequest>,
        resp: grpc::ServerResponseUnarySink<bugs::CreateBugResponse>,
    ) -> grpc::Result<()> {
        if !self.authenticate(req.message.get_token()) {
            let mut response = bugs::CreateBugResponse::new();
            response.set_error(bugs::Error::AUTHENTICATION);
            return resp.finish(response);
        }

        let mut response = bugs::CreateBugResponse::new();
        *response.mut_bug() = self.create_bug(req.message.take_bug());
        resp.finish(response)
    }

    fn update_bug(
        &self,
        _: grpc::ServerHandlerContext,
        mut req: grpc::ServerRequestSingle<bugs::UpdateBugRequest>,
        resp: grpc::ServerResponseUnarySink<bugs::UpdateBugResponse>,
    ) -> grpc::Result<()> {
        if !self.authenticate(req.message.get_token()) {
            let mut response = bugs::UpdateBugResponse::new();
            response.set_error(bugs::Error::AUTHENTICATION);
            return resp.finish(response);
        }

        self.update_bug(req.message.take_bug());
        resp.finish(bugs::UpdateBugResponse::new())
    }
}
