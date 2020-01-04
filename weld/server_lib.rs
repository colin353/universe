extern crate grpc;
extern crate largetable_client;
extern crate weld;
extern crate weld_repo;

use largetable_client::LargeTableClient;
use weld::File;

const HEAD_ID: u64 = 0;
const SUBMITTED: &'static str = "submitted";

#[derive(Clone)]
pub struct WeldServiceHandler<C: LargeTableClient> {
    database: C,
    repo: weld_repo::Repo<C, weld::WeldServerClient>,
}

impl<C: LargeTableClient + Clone> WeldServiceHandler<C> {
    pub fn new(db: C) -> Self {
        let mut handler = WeldServiceHandler {
            database: db.clone(),
            repo: weld_repo::Repo::new(db),
        };
        handler.initialize();
        handler
    }
}

impl<C: LargeTableClient> WeldServiceHandler<C> {
    fn authenticate(&self, _ctx: &grpc::RequestOptions) -> Option<String> {
        Some(String::from("tester"))
    }

    fn initialize(&mut self) {
        // Create an initial change for HEAD.
        self.repo.initialize_head(HEAD_ID);
    }

    pub fn read(&self, file: weld::FileIdentifier) -> File {
        match self
            .repo
            .read(file.get_id(), file.get_filename(), file.get_index())
        {
            Some(f) => f,
            None => File::new(),
        }
    }

    pub fn read_attrs(&self, file: weld::FileIdentifier) -> File {
        match self
            .repo
            .read_attrs(file.get_id(), file.get_filename(), file.get_index())
        {
            Some(f) => f,
            None => File::new(),
        }
    }

    pub fn submit(&self, _username: &str, pending_id: u64) -> weld::SubmitResponse {
        if pending_id == HEAD_ID {
            println!("[submit] Can't submit HEAD, that doesn't make sense");
            return weld::SubmitResponse::new();
        }

        let mut change = match self.repo.get_change(pending_id) {
            Some(c) => c,
            None => {
                println!("[submit] tried to submit not found change: {}", pending_id);
                return weld::SubmitResponse::new();
            }
        };

        if !change.get_found() {
            println!("[submit] no such changelist: {}", &change.get_id());
            return weld::SubmitResponse::new();
        }

        assert!(
            change.get_found(),
            "try to submit not found changelist: {}",
            &change.get_id()
        );

        // Double check that the change is not out of date.
        let most_recent_change = self.get_latest_change();
        println!(
            "[submit] freshness check (got {}, want {})",
            change.get_based_index(),
            most_recent_change.get_submitted_id()
        );
        if change.get_based_index() != most_recent_change.get_submitted_id() {
            println!(
                "[submit] failed, out of date (got {}, want {})",
                change.get_based_index(),
                most_recent_change.get_submitted_id()
            );
            let mut response = weld::SubmitResponse::new();
            response.set_status(weld::SubmitStatus::REQUIRES_SYNC);
            return response;
        }

        // Acquire an index for this submission.
        let id = self.repo.reserve_change_id();

        // Save the change into the submitted changes database.
        change.set_found(true);
        change.set_id(id);
        change.set_submitted_id(id);
        change.set_status(weld::ChangeStatus::SUBMITTED);

        // Write all file changes to HEAD.
        let mut num_changed_files = 0;
        for file in self.repo.list_changed_files(pending_id, 0) {
            self.repo.write(HEAD_ID, file.clone(), id);
            num_changed_files += 1;
        }

        println!(
            "[submit] #{} with {} total changed files",
            id, num_changed_files
        );

        self.database
            .write_proto(SUBMITTED, &index_to_rowname(id), 0, &change);

        self.repo.delete_change(pending_id);

        let mut response = weld::SubmitResponse::new();
        response.set_status(weld::SubmitStatus::OK);
        response.set_id(id);
        response
    }

    pub fn snapshot(&self, username: &str, mut change: weld::Change) -> weld::SnapshotResponse {
        // This change must be based on a change here.
        change.set_is_based_locally(true);
        change.set_author(username.to_owned());
        change.set_last_modified_timestamp(weld::get_timestamp_usec());

        if change.get_status() == weld::ChangeStatus::UNKNOWN {
            change.set_status(weld::ChangeStatus::PENDING);
        }

        assert!(
            self.repo.get_change(change.get_based_id()).is_some(),
            "Must be based on a valid change (tried to base on {}).",
            change.get_based_id()
        );

        // If there's no associated ID, we need to create the repo here first.
        if change.get_id() == 0 {
            let id = self.repo.make_change(change.clone());
            change.set_id(id);
        }

        let mut unchanged_files = std::collections::HashSet::new();
        for file in self.repo.list_changed_files(change.get_id(), 0) {
            unchanged_files.insert(file.get_filename().to_owned());
        }

        for file in change.get_staged_files() {
            self.repo.write(change.get_id(), file.clone(), 0);
            unchanged_files.remove(file.get_filename());
        }

        // Any originally existing changed files which are not listed are
        // to be reverted.
        for filename in unchanged_files.into_iter() {
            let mut f = File::new();
            f.set_filename(filename);
            f.set_reverted(true);
            self.repo.write(change.get_id(), f, 0);
        }

        // Reload any existing data about this change, in case it already exists.
        let mut reloaded_change = self.repo.get_change(change.get_id()).unwrap();
        weld::deserialize_change(
            &weld::serialize_change(&change, false),
            &mut reloaded_change,
        )
        .unwrap();

        reloaded_change.set_based_index(change.get_based_index());
        reloaded_change.set_last_modified_timestamp(change.get_last_modified_timestamp());

        let response = self.repo.snapshot(&reloaded_change);

        reloaded_change.clear_staged_files();
        self.repo
            .update_change(&self.repo.populate_change(reloaded_change.clone()));

        response
    }

    pub fn list_files(&self, file: weld::FileIdentifier) -> weld::ListFilesResponse {
        let mut response = weld::ListFilesResponse::new();
        response.set_files(protobuf::RepeatedField::from_vec(self.repo.list_files(
            file.get_id(),
            file.get_filename(),
            file.get_index(),
        )));
        response
    }

    pub fn list_changes(
        &self,
        _username: &str,
        _req: weld::ListChangesRequest,
    ) -> weld::ListChangesResponse {
        let changes = self
            .repo
            .list_changes()
            .into_iter()
            // Don't respond with HEAD as a change - it shouldn't be counted.
            .filter(|c| c.get_id() != 0)
            .collect::<Vec<_>>();

        let mut response = weld::ListChangesResponse::new();
        response.set_changes(protobuf::RepeatedField::from_vec(changes));
        response
    }

    pub fn get_change(&self, change: weld::Change) -> weld::Change {
        if let Some(c) = self.repo.get_change(change.get_id()) {
            return c;
        }
        if let Some(c) = self
            .database
            .read_proto(SUBMITTED, &index_to_rowname(change.get_id()), 0)
        {
            return c;
        }

        weld::Change::new()
    }

    pub fn get_latest_change(&self) -> weld::Change {
        let mut iter = largetable_client::LargeTableScopedIterator::<'_, weld::Change, _>::new(
            &self.database,
            SUBMITTED.to_owned(),
            String::from(""),
            String::from(""),
            String::from(""),
            0,
        );
        iter.batch_size = 1;
        match iter.next() {
            Some((_, c)) => c,
            None => {
                let mut c = weld::Change::new();
                c.set_submitted_id(1);
                c.set_found(true);
                c
            }
        }
    }

    pub fn get_submitted_changes(
        &self,
        req: &weld::GetSubmittedChangesRequest,
    ) -> weld::GetSubmittedChangesResponse {
        let end = index_to_rowname(req.get_starting_id());
        let start = if req.get_ending_id() != 0 {
            // We want to include the final index, so add one
            index_to_rowname(req.get_ending_id() + 1)
        } else {
            String::new()
        };

        let changes = largetable_client::LargeTableScopedIterator::new(
            &self.database,
            String::from(SUBMITTED),
            String::from(""),
            start,
            end,
            0,
        );
        let mut output = weld::GetSubmittedChangesResponse::new();
        let mut count = 0;
        for (_, change) in changes {
            if req.get_limit() > 0 && count == req.get_limit() {
                break;
            }
            count += 1;
            output.mut_changes().push(change);
        }
        output
    }

    pub fn update_change_metadata(&self, mut req: weld::Change) -> weld::Change {
        let mut ch = match self.repo.get_change(req.get_id()) {
            Some(c) => c,
            None => return weld::Change::new(),
        };

        if !req.get_task_id().is_empty() {
            ch.set_task_id(req.take_task_id());
        }
        if req.get_status() != weld::ChangeStatus::UNKNOWN
            && req.get_status() != weld::ChangeStatus::SUBMITTED
        {
            ch.set_status(req.get_status());
        }
        if req.get_reviewers().len() > 0 {
            ch.set_reviewers(req.take_reviewers());
        }

        self.repo.update_change(&ch);
        ch
    }
}

fn index_to_rowname(index: u64) -> String {
    format!("{:016x}", std::u64::MAX - index)
}

impl<C: LargeTableClient> weld::WeldService for WeldServiceHandler<C> {
    fn read(
        &self,
        m: grpc::RequestOptions,
        req: weld::FileIdentifier,
    ) -> grpc::SingleResponse<weld::File> {
        match self.authenticate(&m) {
            Some(_username) => grpc::SingleResponse::completed(self.read(req)),
            None => grpc::SingleResponse::err(grpc::Error::Other("unauthenticated")),
        }
    }

    fn read_attrs(
        &self,
        m: grpc::RequestOptions,
        req: weld::FileIdentifier,
    ) -> grpc::SingleResponse<weld::File> {
        match self.authenticate(&m) {
            Some(_username) => grpc::SingleResponse::completed(self.read_attrs(req)),
            None => grpc::SingleResponse::err(grpc::Error::Other("unauthenticated")),
        }
    }

    fn list_changes(
        &self,
        m: grpc::RequestOptions,
        req: weld::ListChangesRequest,
    ) -> grpc::SingleResponse<weld::ListChangesResponse> {
        match self.authenticate(&m) {
            Some(username) => grpc::SingleResponse::completed(self.list_changes(&username, req)),
            None => grpc::SingleResponse::err(grpc::Error::Other("unauthenticated")),
        }
    }

    fn submit(
        &self,
        m: grpc::RequestOptions,
        req: weld::Change,
    ) -> grpc::SingleResponse<weld::SubmitResponse> {
        match self.authenticate(&m) {
            Some(username) => grpc::SingleResponse::completed(self.submit(&username, req.get_id())),
            None => grpc::SingleResponse::err(grpc::Error::Other("unauthenticated")),
        }
    }

    fn snapshot(
        &self,
        m: grpc::RequestOptions,
        req: weld::Change,
    ) -> grpc::SingleResponse<weld::SnapshotResponse> {
        match self.authenticate(&m) {
            Some(username) => grpc::SingleResponse::completed(self.snapshot(&username, req)),
            None => grpc::SingleResponse::err(grpc::Error::Other("unauthenticated")),
        }
    }

    fn get_change(
        &self,
        m: grpc::RequestOptions,
        req: weld::Change,
    ) -> grpc::SingleResponse<weld::Change> {
        match self.authenticate(&m) {
            Some(_) => grpc::SingleResponse::completed(self.get_change(req)),
            None => grpc::SingleResponse::err(grpc::Error::Other("unauthenticated")),
        }
    }

    fn get_latest_change(
        &self,
        m: grpc::RequestOptions,
        _req: weld::GetLatestChangeRequest,
    ) -> grpc::SingleResponse<weld::Change> {
        match self.authenticate(&m) {
            Some(_) => grpc::SingleResponse::completed(self.get_latest_change()),
            None => grpc::SingleResponse::err(grpc::Error::Other("unauthenticated")),
        }
    }

    fn list_files(
        &self,
        m: grpc::RequestOptions,
        req: weld::FileIdentifier,
    ) -> grpc::SingleResponse<weld::ListFilesResponse> {
        match self.authenticate(&m) {
            Some(_) => grpc::SingleResponse::completed(self.list_files(req)),
            None => grpc::SingleResponse::err(grpc::Error::Other("unauthenticated")),
        }
    }

    fn get_submitted_changes(
        &self,
        m: grpc::RequestOptions,
        req: weld::GetSubmittedChangesRequest,
    ) -> grpc::SingleResponse<weld::GetSubmittedChangesResponse> {
        match self.authenticate(&m) {
            Some(_) => grpc::SingleResponse::completed(self.get_submitted_changes(&req)),
            None => grpc::SingleResponse::err(grpc::Error::Other("unauthenticated")),
        }
    }

    fn update_change_metadata(
        &self,
        m: grpc::RequestOptions,
        req: weld::Change,
    ) -> grpc::SingleResponse<weld::Change> {
        match self.authenticate(&m) {
            Some(_) => grpc::SingleResponse::completed(self.update_change_metadata(req)),
            None => grpc::SingleResponse::err(grpc::Error::Other("unauthenticated")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    extern crate largetable_test;
    extern crate protobuf;

    impl WeldServiceHandler<largetable_test::LargeTableMockClient> {
        pub fn create_mock() -> Self {
            let db = largetable_test::LargeTableMockClient::new();
            let mut handler = WeldServiceHandler {
                database: db.clone(),
                repo: weld_repo::Repo::new(db),
            };
            handler.initialize();
            handler
        }
    }

    fn test_file(filename: &str, contents: &str) -> weld::File {
        let mut f = weld::File::new();
        f.set_filename(filename.to_owned());
        f.set_contents(contents.to_owned().into_bytes());
        f
    }

    fn test_ident(id: u64, path: &str, index: u64) -> weld::FileIdentifier {
        let mut f = weld::FileIdentifier::new();
        f.set_id(id);
        f.set_filename(path.to_owned());
        f.set_index(index);
        f
    }

    #[test]
    fn test_write() {
        // Check that the file is not written.
        let handler = WeldServiceHandler::create_mock();

        assert_eq!(
            handler
                .read(test_ident(HEAD_ID, "/test/config.txt", 0))
                .get_found(),
            false
        );

        let mut change = weld::Change::new();
        change.set_based_index(1);
        change
            .mut_staged_files()
            .push(test_file("/test/config.txt", "working = true"));

        let change_id = handler.snapshot("tester", change).get_change_id();
        handler.submit("tester", change_id);

        assert_eq!(
            handler
                .read(test_ident(HEAD_ID, "/test/config.txt", 0))
                .get_found(),
            true
        );
    }

    #[test]
    fn test_snapshot() {
        // Check that the file is not written.
        let handler = WeldServiceHandler::create_mock();

        let mut change = weld::Change::new();
        change
            .mut_staged_files()
            .push(test_file("/test/config.txt", "working = true"));

        let id = handler.snapshot("tester", change).get_change_id();

        assert_eq!(
            handler
                .read(test_ident(id, "/test/config.txt", 0))
                .get_found(),
            true
        );
    }

    #[test]
    fn test_inherit_changes() {
        let handler = WeldServiceHandler::create_mock();

        // Write /test/config.txt and submit it to head.
        let mut change = weld::Change::new();
        change.set_based_index(1);
        change
            .mut_staged_files()
            .push(test_file("/test/config.txt", "working = true"));

        let change_id = handler.snapshot("tester", change).get_change_id();
        handler.submit("tester", change_id);

        // Now create a change based on the previous change.
        let mut change = weld::Change::new();
        change
            .mut_staged_files()
            .push(test_file("/test/README.txt", "hello world"));

        let change_id = handler.snapshot("tester", change).get_change_id();

        // Now, should be able to read the original file via the new snapshot.
        assert_eq!(
            handler
                .read(test_ident(change_id, "/test/README.txt", 0))
                .get_found(),
            true
        );
        assert_eq!(
            handler
                .read(test_ident(change_id, "/test/config.txt", 0))
                .get_found(),
            true
        );
    }

    #[test]
    fn test_list_files() {
        // Check that the file is not written.
        let handler = WeldServiceHandler::create_mock();

        // Write /test/config.txt and submit it to head.
        let mut change = weld::Change::new();
        change.set_based_index(1);
        change
            .mut_staged_files()
            .push(test_file("/test/config.txt", "working = true"));

        change
            .mut_staged_files()
            .push(test_file("/test/README.txt", "hello, world"));

        let change_id = handler.snapshot("tester", change).get_change_id();
        handler.submit("tester", change_id);

        let results = handler
            .list_files(test_ident(HEAD_ID, "/test", 0))
            .get_files()
            .iter()
            .map(|x| x.get_filename().to_owned())
            .collect::<Vec<_>>();
        assert_eq!(results, vec!["/test/README.txt", "/test/config.txt"]);
    }

    #[test]
    fn test_list_files_based_change() {
        let handler = WeldServiceHandler::create_mock();

        // Write /test/config.txt and submit it to head.
        let mut change = weld::Change::new();
        change.set_based_index(1);
        change
            .mut_staged_files()
            .push(test_file("/test/config.txt", "working = true"));
        change
            .mut_staged_files()
            .push(test_file("/test/README.txt", "hello, world"));

        let change_id = handler.snapshot("tester", change).get_change_id();
        handler.submit("tester", change_id);

        // Write some more files and snapshot them.
        let mut change = weld::Change::new();
        change
            .mut_staged_files()
            .push(test_file("/test/abstract.txt", "test content"));
        change
            .mut_staged_files()
            .push(test_file("/test/zebra.txt", "the stripiest animal"));

        let change_id = handler.snapshot("tester", change).get_change_id();

        let results = handler
            .list_files(test_ident(change_id, "/test", 0))
            .get_files()
            .iter()
            .map(|x| x.get_filename().to_owned())
            .collect::<Vec<_>>();
        assert_eq!(
            results,
            vec![
                "/test/README.txt",
                "/test/abstract.txt",
                "/test/config.txt",
                "/test/zebra.txt"
            ]
        );
    }

    #[test]
    fn test_get_latest_change() {
        // Check that the file is not written.
        let handler = WeldServiceHandler::create_mock();

        // Write /test/config.txt and submit it to head.
        let mut change = weld::Change::new();
        change.set_based_index(1);
        change
            .mut_staged_files()
            .push(test_file("/test/config.txt", "working = true"));
        change
            .mut_staged_files()
            .push(test_file("/test/README.txt", "hello, world"));

        let change_id = handler.snapshot("tester", change).get_change_id();
        handler.submit("tester", change_id);

        let mut c = weld::Change::new();
        c.set_id(change_id);
        c.set_based_index(1);
        let result = handler.get_change(c);
        assert_eq!(result.get_found(), true);
        assert_eq!(result.get_id(), 2);

        let result = handler.get_latest_change();
        assert_eq!(result.get_found(), true);
        assert_eq!(result.get_id(), 3);

        // Write /test/config.txt and submit it to head.
        let mut change = weld::Change::new();
        change.set_based_index(3);
        change
            .mut_staged_files()
            .push(test_file("/test/config.txt", "test123"));

        let change_id = handler.snapshot("tester", change).get_change_id();
        handler.submit("tester", change_id);

        let result = handler.get_latest_change();
        assert_eq!(result.get_found(), true);
        assert_eq!(result.get_id(), 5);
    }

    #[test]
    fn test_get_pending_change() {
        let handler = WeldServiceHandler::create_mock();

        // Write /test/config.txt and submit it to head.
        let mut change = weld::Change::new();
        change.set_based_index(1);
        change
            .mut_staged_files()
            .push(test_file("/test/config.txt", "working = false"));

        let change_id = handler.snapshot("tester", change).get_change_id();
        handler.submit("tester", change_id);

        // Make a change that updates the same file.
        let mut change = weld::Change::new();
        change
            .mut_staged_files()
            .push(test_file("/test/config.txt", "working = true"));

        let change_id = handler.snapshot("tester", change).get_change_id();

        // Now the change should be fully populated:
        let mut c = weld::Change::new();
        c.set_id(change_id);
        let change = handler.get_change(c);

        // The staged files should not be stored.
        assert_eq!(change.get_staged_files().len(), 0);

        // The order could be random, so let's store the output in a hash map.
        let mut map = HashMap::new();
        for c in change.get_changes() {
            map.insert(String::from(c.get_filename()), c.clone());
        }

        // Check that the expected number of changes are there.
        assert_eq!(map.len(), 1);
        assert_eq!(
            map.get("/test/config.txt").unwrap().get_snapshots().len(),
            2
        );

        // Check that the revision from the based repo is in there.
        assert_eq!(
            std::str::from_utf8(
                map.get("/test/config.txt")
                    .unwrap()
                    .get_snapshots()
                    .get(0)
                    .unwrap()
                    .get_contents()
            )
            .unwrap(),
            "working = false"
        );

        // Check that the revision from the based repo is in there.
        assert_eq!(
            std::str::from_utf8(
                map.get("/test/config.txt")
                    .unwrap()
                    .get_snapshots()
                    .get(1)
                    .unwrap()
                    .get_contents()
            )
            .unwrap(),
            "working = true"
        );
    }

    #[test]
    fn test_list_submitted_changes() {
        let handler = WeldServiceHandler::create_mock();

        // Write /test/config.txt and submit it to head.
        let mut change = weld::Change::new();
        change.set_based_index(1);
        change
            .mut_staged_files()
            .push(test_file("/test/config.txt", "working = true"));

        let change_id = handler.snapshot("tester", change).get_change_id();
        handler.submit("tester", change_id);

        // Write /test/config.txt and submit it to head.
        let mut change = weld::Change::new();
        change.set_based_index(3);
        change
            .mut_staged_files()
            .push(test_file("/test/test.txt", "working = true"));

        let change_id_2 = handler.snapshot("tester", change).get_change_id();
        handler.submit("tester", change_id_2);

        // Write /test/config.txt and submit it to head.
        let mut change = weld::Change::new();
        change.set_based_index(5);
        change
            .mut_staged_files()
            .push(test_file("/test/test.txt", "working = true"));

        let change_id = handler.snapshot("tester", change).get_change_id();
        handler.submit("tester", change_id);

        // Read off all submitted changes
        let mut req = weld::GetSubmittedChangesRequest::new();
        req.set_ending_id(4);
        let resp = handler.get_submitted_changes(&req);
        assert_eq!(resp.get_changes().len(), 2);
        assert!(resp.get_changes()[0].get_submitted_id() > 0);
        let req = weld::ListChangesRequest::new();
        let resp = handler.list_changes("", req);
        assert_eq!(resp.get_changes().len(), 0);
    }

    #[test]
    fn test_out_of_date_change_submission() {
        let handler = WeldServiceHandler::create_mock();

        // Write /test/config.txt and submit it to head.
        let mut change = weld::Change::new();
        change.set_based_index(1);
        change
            .mut_staged_files()
            .push(test_file("/test/config.txt", "working = true"));

        let change_id = handler.snapshot("tester", change).get_change_id();
        handler.submit("tester", change_id);

        // Write /test/config.txt and submit it to head.
        let mut change = weld::Change::new();
        change.set_based_index(3);
        change
            .mut_staged_files()
            .push(test_file("/test/test.txt", "working = true"));

        let change_id_2 = handler.snapshot("tester", change).get_change_id();
        let result = handler.submit("tester", change_id_2);
        // Expect that this will submit OK, since it's based on #2
        assert_eq!(result.get_status(), weld::SubmitStatus::OK);

        // Write /test/config.txt and submit it to head.
        let mut change = weld::Change::new();
        change.set_based_index(2);
        change
            .mut_staged_files()
            .push(test_file("/test/test.txt", "working = true"));

        let change_id = handler.snapshot("tester", change).get_change_id();
        let result = handler.submit("tester", change_id);
        // This should fail since it's based on #2, which is out of date
        assert_eq!(result.get_status(), weld::SubmitStatus::REQUIRES_SYNC);
    }
}
