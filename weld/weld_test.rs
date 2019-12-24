extern crate largetable_test;
extern crate weld;
extern crate weld_server_lib;

use std::sync::Arc;

#[derive(Clone)]
pub struct WeldServerTestClient {
    client: Arc<weld_server_lib::WeldServiceHandler<largetable_test::LargeTableMockClient>>,
    username: String,
}

impl WeldServerTestClient {
    pub fn new(username: String) -> Self {
        let db = largetable_test::LargeTableMockClient::new();
        WeldServerTestClient {
            client: Arc::new(weld_server_lib::WeldServiceHandler::new(db)),
            username: username,
        }
    }
}

impl weld::WeldServer for WeldServerTestClient {
    fn read(&self, req: weld::FileIdentifier) -> weld::File {
        self.client.read(req)
    }
    fn read_attrs(&self, req: weld::FileIdentifier) -> weld::File {
        self.client.read_attrs(req)
    }
    fn submit(&self, req: weld::Change) -> weld::SubmitResponse {
        self.client.submit(&self.username, req.get_id())
    }
    fn snapshot(&self, req: weld::Change) -> weld::SnapshotResponse {
        self.client.snapshot(&self.username, req)
    }
    fn get_change(&self, req: weld::Change) -> weld::Change {
        self.client.get_change(req)
    }
    fn list_changes(&self) -> Vec<weld::Change> {
        self.client
            .list_changes(&self.username, weld::ListChangesRequest::new())
            .take_changes()
            .into_vec()
    }
    fn get_latest_change(&self) -> weld::Change {
        self.client.get_latest_change()
    }
    fn list_files(&self, req: weld::FileIdentifier) -> Vec<weld::File> {
        self.client.list_files(req).take_files().into_vec()
    }
    fn get_submitted_changes(&self, req: weld::GetSubmittedChangesRequest) -> Vec<weld::Change> {
        self.client
            .get_submitted_changes(&req)
            .take_changes()
            .into_vec()
    }
    fn register_task_for_change(&self, req: weld::Change) {
        self.client.register_task_for_change(req);
    }
}
