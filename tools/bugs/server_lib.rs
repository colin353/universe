extern crate bugs_grpc_rust as bugs;
extern crate grpc;
extern crate largetable_client;

use largetable_client::LargeTableClient;

const BUG_IDS: &'static str = "bug_ids";
const ALL_BUGS: &'static str = "all_bugs";

#[derive(Clone)]
pub struct BugServiceHandler<C: LargeTableClient> {
    database: C,
}

fn bug_rowname(s: bugs::BugStatus) -> String {
    format!("bugs::{:?}", s)
}

fn bug_colname(id: u64) -> String {
    id.to_string()
}

impl<C: LargeTableClient + Clone> BugServiceHandler<C> {
    pub fn new(db: C) -> Self {
        Self { database: db }
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
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate largetable_test;

    fn create_test_handler() -> BugServiceHandler<largetable_test::LargeTableMockClient> {
        let db = largetable_test::LargeTableMockClient::new();
        BugServiceHandler::new(db)
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
