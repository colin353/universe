extern crate futures;
extern crate grpc;
extern crate httpbis;
extern crate native_tls;
extern crate protobuf;
extern crate time;
extern crate tls_api;
extern crate weld_grpc_rust;

pub use weld_grpc_rust::WeldLocalService;
pub use weld_grpc_rust::WeldService;
pub use weld_grpc_rust::*;

use grpc::ClientStub;
use grpc::ClientStubExt;

use tls_api::TlsConnector;
use tls_api::TlsConnectorBuilder;

use std::net::SocketAddr;
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct WeldServerClient {
    client: Arc<weld_grpc_rust::WeldServiceClient>,
    permanent_token: Arc<RwLock<Option<String>>>,
    temporary_token: Arc<RwLock<Option<String>>>,
}

#[derive(Clone)]
pub struct WeldLocalClient {
    client: Arc<weld_grpc_rust::WeldLocalServiceClient>,
}

pub trait WeldServer {
    fn read(&self, req: weld::FileIdentifier) -> weld::File;
    fn read_attrs(&self, req: weld::FileIdentifier) -> weld::File;
    fn submit(&self, req: weld::Change) -> weld::SubmitResponse;
    fn snapshot(&self, req: weld::Change) -> weld::SnapshotResponse;
    fn get_change(&self, req: weld::Change) -> weld::Change;
    fn list_changes(&self) -> Vec<Change>;
    fn get_latest_change(&self) -> weld::Change;
    fn list_files(&self, req: weld::FileIdentifier) -> Vec<File>;
    fn get_submitted_changes(&self, req: weld::GetSubmittedChangesRequest) -> Vec<Change>;
    fn update_change_metadata(&self, req: weld::Change);
}

impl WeldServerClient {
    pub fn new(hostname: &str, username: String, port: u16) -> Self {
        let c = WeldServerClient {
            client: Arc::new(
                weld_grpc_rust::WeldServiceClient::new_plain(hostname, port, Default::default())
                    .unwrap(),
            ),
            permanent_token: Arc::new(RwLock::new(None)),
            temporary_token: Arc::new(RwLock::new(None)),
        };
        c.load_token();
        c
    }

    pub fn set_permanent_token(&self, token: String) {
        *self.permanent_token.write().unwrap() = Some(token);
    }

    pub fn new_tls(hostname: &str, port: u16) -> Self {
        let grpc_client = grpc_tls::make_tls_client(hostname, port);
        let c = WeldServerClient {
            client: Arc::new(weld_grpc_rust::WeldServiceClient::with_client(Arc::new(
                grpc_client,
            ))),
            permanent_token: Arc::new(RwLock::new(None)),
            temporary_token: Arc::new(RwLock::new(None)),
        };
        c.load_token();
        c
    }

    fn opts(&self) -> grpc::RequestOptions {
        grpc::RequestOptions::new()
    }

    fn get_token(&self) -> String {
        if let Some(s) = self.permanent_token.read().unwrap().as_ref() {
            return s.to_owned();
        }

        if let Some(s) = self.temporary_token.read().unwrap().as_ref() {
            return s.to_owned();
        }

        let temp_token = cli::load_auth();
        if !temp_token.is_empty() {
            *self.temporary_token.write().unwrap() = Some(temp_token.clone());
        } else {
            eprintln!("couldn't load auth token! have you run prodaccess?");
        }

        return temp_token;
    }

    fn load_token(&self) {
        let temp_token = cli::load_auth();
        if !temp_token.is_empty() {
            *self.temporary_token.write().unwrap() = Some(temp_token.clone());
        } else {
            eprintln!("auth failed! have you run prodaccess?");
        }
    }
}

impl WeldServer for WeldServerClient {
    fn read(&self, mut req: weld::FileIdentifier) -> weld::File {
        req.set_auth_token(self.get_token());

        if let Ok(x) = wait(self.client.read(self.opts(), req)) {
            return x;
        }

        self.load_token();
        panic!("read request failed!");
    }

    fn read_attrs(&self, mut req: weld::FileIdentifier) -> weld::File {
        req.set_auth_token(self.get_token());

        if let Ok(x) = wait(self.client.read_attrs(self.opts(), req)) {
            return x;
        }

        self.load_token();
        panic!("read_attrs request failed!");
    }

    fn submit(&self, mut req: weld::Change) -> weld::SubmitResponse {
        req.set_auth_token(self.get_token());
        if let Ok(x) = wait(self.client.submit(self.opts(), req)) {
            return x;
        }

        self.load_token();
        panic!("read_attrs request failed!");
    }

    fn snapshot(&self, mut req: weld::Change) -> weld::SnapshotResponse {
        req.set_auth_token(self.get_token());
        if let Ok(x) = wait(self.client.snapshot(self.opts(), req)) {
            return x;
        }

        self.load_token();
        panic!("snapshot request failed!");
    }

    fn get_change(&self, mut req: weld::Change) -> weld::Change {
        req.set_auth_token(self.get_token());
        if let Ok(x) = wait(self.client.get_change(self.opts(), req)) {
            return x;
        }

        self.load_token();
        panic!("get_change request failed!");
    }

    fn list_changes(&self) -> Vec<Change> {
        let mut req = weld::ListChangesRequest::new();
        req.set_auth_token(self.get_token());
        if let Ok(mut x) = wait(self.client.list_changes(self.opts(), req)) {
            return x.take_changes().into_vec();
        }
        self.load_token();
        panic!("list_changes request failed!");
    }

    fn get_latest_change(&self) -> weld::Change {
        let mut req = weld::GetLatestChangeRequest::new();
        req.set_auth_token(self.get_token());
        if let Ok(x) = wait(self.client.get_latest_change(self.opts(), req)) {
            return x;
        }

        self.load_token();
        panic!("get_latest_change request failed!");
    }

    fn list_files(&self, mut req: weld::FileIdentifier) -> Vec<File> {
        req.set_auth_token(self.get_token());
        if let Ok(mut x) = wait(self.client.list_files(self.opts(), req)) {
            return x.take_files().into_vec();
        }
        self.load_token();
        panic!("list_files request failed!");
    }

    fn get_submitted_changes(&self, mut req: weld::GetSubmittedChangesRequest) -> Vec<Change> {
        req.set_auth_token(self.get_token());
        if let Ok(mut x) = wait(self.client.get_submitted_changes(self.opts(), req)) {
            return x.take_changes().into_vec();
        }
        self.load_token();
        panic!("get_submitted_changes request failed!");
    }

    fn update_change_metadata(&self, mut req: weld::Change) {
        req.set_auth_token(self.get_token());
        if let Err(_) = wait(self.client.update_change_metadata(self.opts(), req)) {
            self.load_token();
            panic!("update_change_metadata request failed!");
        }
    }
}

impl WeldLocalClient {
    pub fn new(hostname: &str, port: u16) -> Self {
        WeldLocalClient {
            client: Arc::new(
                weld_grpc_rust::WeldLocalServiceClient::new_plain(
                    hostname,
                    port,
                    Default::default(),
                )
                .unwrap(),
            ),
        }
    }

    fn opts(&self) -> grpc::RequestOptions {
        grpc::RequestOptions::new()
    }

    pub fn make_change(&self, req: weld::Change) -> weld::Change {
        wait(self.client.make_change(self.opts(), req)).expect("rpc")
    }

    pub fn read(&self, req: weld::FileIdentifier) -> weld::File {
        wait(self.client.read(self.opts(), req)).expect("rpc")
    }

    pub fn write(&self, req: weld::WriteRequest) {
        wait(self.client.write(self.opts(), req)).expect("rpc");
    }

    pub fn delete(&self, req: weld::FileIdentifier) {
        wait(self.client.delete(self.opts(), req)).expect("rpc");
    }

    pub fn get_change(&self, req: weld::GetChangeRequest) -> weld::Change {
        wait(self.client.get_change(self.opts(), req)).expect("rpc")
    }

    pub fn list_files(&self, req: weld::FileIdentifier) -> Vec<File> {
        wait(self.client.list_files(self.opts(), req))
            .expect("rpc")
            .take_files()
            .into_vec()
    }

    pub fn list_changes(&self) -> Vec<Change> {
        let req = weld::ListChangesRequest::new();
        wait(self.client.list_changes(self.opts(), req))
            .expect("rpc")
            .take_changes()
            .into_vec()
    }

    pub fn snapshot(&self, req: weld::Change) -> weld::SnapshotResponse {
        wait(self.client.snapshot(self.opts(), req)).expect("rpc")
    }

    pub fn submit(&self, req: weld::Change) -> weld::SubmitResponse {
        wait(self.client.submit(self.opts(), req)).expect("rpc")
    }

    pub fn lookup_friendly_name(&self, name: String) -> Option<u64> {
        let mut req = weld::LookupFriendlyNameRequest::new();
        req.set_friendly_name(name);
        match wait(self.client.lookup_friendly_name(self.opts(), req))
            .expect("rpc")
            .get_id()
        {
            0 => None,
            x => Some(x),
        }
    }

    pub fn get_patch(&self, req: weld::Change) -> String {
        wait(self.client.get_patch(self.opts(), req))
            .expect("rpc")
            .get_patch()
            .to_owned()
    }

    pub fn sync(&self, req: weld::SyncRequest) -> weld::SyncResponse {
        wait(self.client.sync(self.opts(), req)).expect("rpc")
    }

    pub fn run_build(&self, req: weld::RunBuildRequest) -> weld::RunBuildResponse {
        wait(self.client.run_build(self.opts(), req)).expect("rpc")
    }

    pub fn run_build_query(&self, req: weld::RunBuildQueryRequest) -> weld::RunBuildQueryResponse {
        wait(self.client.run_build_query(self.opts(), req)).expect("rpc")
    }

    pub fn publish_file(&self, req: weld::PublishFileRequest) -> weld::PublishFileResponse {
        wait(self.client.publish_file(self.opts(), req)).expect("rpc")
    }

    pub fn apply_patch(&self, req: weld::ApplyPatchRequest) -> weld::ApplyPatchResponse {
        wait(self.client.apply_patch(self.opts(), req)).expect("rpc")
    }

    pub fn delete_change(&self, req: weld::Change) -> weld::DeleteResponse {
        wait(self.client.delete_change(self.opts(), req)).expect("rpc")
    }

    pub fn clean_submitted_changes(&self) -> weld::CleanSubmittedChangesResponse {
        wait(
            self.client
                .clean_submitted_changes(self.opts(), weld::CleanSubmittedChangesRequest::new()),
        )
        .expect("rpc")
    }
}

pub fn get_timestamp_usec() -> u64 {
    let tm = time::now_utc().to_timespec();
    (tm.sec as u64) * 1_000_000 + ((tm.nsec / 1000) as u64)
}

pub fn file_id(id: u64, filename: String, index: u64) -> weld::FileIdentifier {
    let mut fid = weld::FileIdentifier::new();
    fid.set_id(id);
    fid.set_filename(filename);
    fid.set_index(index);
    fid
}

pub fn change(id: u64) -> weld::Change {
    let mut c = weld::Change::new();
    c.set_id(id);
    c
}

pub fn deserialize_change(input: &str, change: &mut weld::Change) -> Result<(), String> {
    let mut description = Vec::new();
    for line in input.split("\n") {
        let trimmed = line.trim();

        // Ignore comment lines
        if trimmed.starts_with("#") {
            continue;
        }

        // Add reviewers
        if trimmed.starts_with("R=") {
            change.mut_reviewers().clear();
            for name in trimmed.trim_start_matches("R=").split(",") {
                change.mut_reviewers().push(name.trim().to_owned());
            }
            continue;
        }

        if trimmed.starts_with("ARCHIVE=true") {
            change.set_status(weld::ChangeStatus::ARCHIVED);
        }

        if trimmed.starts_with("ARCHIVE=false") {
            change.set_status(weld::ChangeStatus::PENDING);
        }

        // If the line is too full of spaces, strip them.
        if line.starts_with("   ") {
            description.push(trimmed);
            continue;
        }

        description.push(line);
    }

    // Remove starting and trailing newlines.
    let mut sliced_desc = description.as_slice();
    while let Some(&"") = sliced_desc.first() {
        sliced_desc = &sliced_desc[1..];
    }

    while let Some(&"") = sliced_desc.last() {
        sliced_desc = &sliced_desc[..sliced_desc.len() - 1];
    }

    // If the description is empty, don't update the existing description.
    if sliced_desc.len() > 0 {
        change.set_description(sliced_desc.join("\n"));
    }

    Ok(())
}

pub fn summarize_change(change: &weld::Change) -> String {
    let mut summary = match change.get_description().lines().next() {
        Some(t) => t.to_owned(),
        None => return String::from(""),
    };
    summary.truncate(80);
    summary
}

pub fn serialize_change(change: &weld::Change, with_instructions: bool) -> String {
    let mut output = change.get_description().to_owned();

    let mut annotations = Vec::new();
    if change.get_reviewers().len() > 0 {
        annotations.push(format!("R={}", change.get_reviewers().join(",")));
    }

    if annotations.len() > 0 {
        output.push_str("\n\n");
        output.push_str(&annotations.join("\n"));
    }

    if with_instructions {
        output.push_str(
            "

# Write description above. Lines starting with # will be ignored.
# Add annotations, e.g.
#
# R=xyz
#
# to set special fields.",
        );
    }

    output
}

pub fn summarize_change_description<'a>(description: &'a str) -> &'a str {
    description.lines().next().unwrap_or("")
}

pub fn render_change_description(description: &str) -> String {
    let mut output = String::new();
    let mut chunk = String::new();
    let mut is_list = false;
    let mut is_first_line_of_chunk = true;
    for line in description.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("- ") {
            if chunk.is_empty() {
                chunk = line.to_owned();
                continue;
            }

            if is_list {
                output += &format!("<li>{}</li>", escape::Escape(&chunk));
            } else {
                output += &format!("<p>{}</p>", escape::Escape(&chunk));
            }
            chunk = line.to_owned();
            is_first_line_of_chunk = true;
            is_list = false;
            continue;
        }

        if is_first_line_of_chunk && line.starts_with("- ") {
            is_list = true;
        }
        is_first_line_of_chunk = false;

        chunk += " ";
        chunk += line;
    }

    if is_list {
        output += &format!("<li>{}</li>", escape::Escape(&chunk));
    } else {
        output += &format!("<p>{}</p>", escape::Escape(&chunk));
    }

    output
}

pub fn should_ignore_file(filename: &str) -> bool {
    if filename.ends_with(".swx") {
        return true;
    }
    if filename.ends_with(".swpx") {
        return true;
    }
    if filename.ends_with(".swp") {
        return true;
    }
    if filename.ends_with(".swo") {
        return true;
    }
    if filename.ends_with("~") {
        return true;
    }
    // Filenames with these characters cause problems with fuse for some reason
    if filename.contains("]") || filename.contains("[") {
        return true;
    }

    false
}

pub fn files_are_functionally_the_same(f1: &weld::File, f2: &weld::File) -> bool {
    if f1.get_filename() != f2.get_filename() {
        return false;
    }

    if f1.get_directory() != f2.get_directory() {
        return false;
    }

    if f1.get_contents() != f2.get_contents() {
        return false;
    }

    if f1.get_deleted() != f2.get_deleted() {
        return false;
    }

    if f1.get_found() != f2.get_found() {
        return false;
    }

    true
}

pub fn get_changed_files(change: &weld::Change) -> Vec<&weld::File> {
    // Figure out which files were changed and save them as artifacts
    let maybe_last_snapshot = change
        .get_changes()
        .iter()
        .filter_map(|c| c.get_snapshots().iter().map(|x| x.get_snapshot_id()).max())
        .max();

    let last_snapshot_id = match maybe_last_snapshot {
        Some(x) => x,
        None => return Vec::new(),
    };

    change
        .get_changes()
        .iter()
        .filter_map(|h| {
            h.get_snapshots()
                .iter()
                .filter(|x| x.get_snapshot_id() == last_snapshot_id)
                .next()
        })
        .filter(|f| !f.get_reverted())
        .collect()
}

pub fn get_changed_file<'a>(filename: &str, change: &'a weld::Change) -> Option<&'a weld::File> {
    get_changed_files(change)
        .into_iter()
        .find(|f| f.get_filename() == filename)
}

fn wait<T: Send + Sync>(resp: grpc::SingleResponse<T>) -> Result<T, grpc::Error> {
    futures::executor::block_on(resp.join_metadata_result()).map(|r| r.1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ignore_files() {
        assert!(should_ignore_file("filename.swp"));
        assert!(should_ignore_file("filename.swo"));
        assert!(should_ignore_file("filename~"));
        assert!(!should_ignore_file("file.txt"));
        assert!(!should_ignore_file("file.swot"));
    }

    #[test]
    fn test_summarize() {
        let d = "Hello, world\nAnother line";
        assert_eq!(summarize_change_description(d), "Hello, world");
    }

    #[test]
    fn test_render() {
        let d = "Hello, world\n\nAnother line\n\n - Bullet one\n - Bullet two\n";
        assert_eq!(
            render_change_description(d),
            "<p> Hello, world</p><p> Another line</p><p>- Bullet one</p><p>- Bullet two</p>"
        );
    }

    #[test]
    fn test_deserialize() {
        let mut c = weld::Change::new();

        let text = "
            This is a change description
            R=person1,person2
        ";
        deserialize_change(text, &mut c).unwrap();

        assert_eq!(c.get_description(), "This is a change description");

        assert_eq!(c.get_reviewers(), &["person1", "person2"]);
    }

    #[test]
    fn test_serialize() {
        let mut c = weld::Change::new();
        c.set_description(String::from("I'm a description"));
        c.mut_reviewers().push(String::from("colinmerkel"));
        c.mut_reviewers().push(String::from("tester"));

        let expected = String::from("I'm a description\n\nR=colinmerkel,tester");

        assert_eq!(serialize_change(&c, false), expected);
    }

    #[test]
    fn test_serialize_deserialize() {
        let mut input = weld::Change::new();
        input.set_description(String::from("I'm a description\nwith some more lines"));
        input.mut_reviewers().push(String::from("colinmerkel"));
        input.mut_reviewers().push(String::from("tester"));

        let mut output = weld::Change::new();
        deserialize_change(&serialize_change(&input, true), &mut output).unwrap();

        assert_eq!(input, output);
    }

    #[test]
    fn test_no_overwrite_existing() {
        let mut input = weld::Change::new();
        input.set_description(String::from("I'm a description\nwith some more lines"));
        input.mut_reviewers().push(String::from("colinmerkel"));
        input.mut_reviewers().push(String::from("tester"));

        let mut output = input.clone();
        deserialize_change("", &mut output).unwrap();

        assert_eq!(input, output);
    }
}
