extern crate bugs_grpc_rust as bugs;

use bugs::BugService;
use std::sync::Arc;

pub struct BugClient {
    client: Arc<bugs::BugServiceClient>,
    token: String,
}

impl BugClient {
    pub fn new(hostname: &str, port: u16, token: String) -> Self {
        BugClient {
            client: Arc::new(
                bugs::BugServiceClient::new_plain(hostname, port, Default::default()).unwrap(),
            ),
            token: token,
        }
    }

    pub fn get_bugs(&self, status: bugs::BugStatus) -> Result<Vec<bugs::Bug>, bugs::Error> {
        let mut req = bugs::GetBugsRequest::new();
        req.set_token(self.token.clone());
        req.set_status(status);

        let mut response = self
            .client
            .get_bugs(std::default::Default::default(), req)
            .wait()
            .expect("rpc")
            .1;

        if response.get_error() == bugs::Error::NONE {
            return Ok(response.take_bugs().into_vec());
        }
        Err(response.get_error())
    }

    pub fn get_bug(&self, id: u64) -> Result<Option<bugs::Bug>, bugs::Error> {
        let mut req = bugs::GetBugRequest::new();
        req.set_token(self.token.clone());
        req.mut_bug().set_id(id);

        let mut response = self
            .client
            .get_bug(std::default::Default::default(), req)
            .wait()
            .expect("rpc")
            .1;

        if response.get_error() == bugs::Error::NONE {
            if !response.get_found() {
                return Ok(None);
            }
            return Ok(Some(response.take_bug()));
        }

        Err(response.get_error())
    }

    pub fn create_bug(&self, b: bugs::Bug) -> Result<bugs::Bug, bugs::Error> {
        let mut req = bugs::CreateBugRequest::new();
        req.set_token(self.token.clone());
        *req.mut_bug() = b;

        let mut response = self
            .client
            .create_bug(std::default::Default::default(), req)
            .wait()
            .expect("rpc")
            .1;

        if response.get_error() == bugs::Error::NONE {
            return Ok(response.take_bug());
        }

        Err(response.get_error())
    }

    pub fn update_bug(&self, b: bugs::Bug) -> Result<(), bugs::Error> {
        let mut req = bugs::UpdateBugRequest::new();
        req.set_token(self.token.clone());
        *req.mut_bug() = b;

        let response = self
            .client
            .update_bug(std::default::Default::default(), req)
            .wait()
            .expect("rpc")
            .1;

        if response.get_error() == bugs::Error::NONE {
            return Ok(());
        }

        Err(response.get_error())
    }
}
