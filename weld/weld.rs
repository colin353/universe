extern crate futures;
extern crate grpc;
extern crate httpbis;
extern crate native_tls;
extern crate protobuf;
extern crate time;
extern crate tls_api;
extern crate tls_api_native_tls;
extern crate weld_grpc_rust;

pub use weld_grpc_rust::WeldLocalService;
pub use weld_grpc_rust::WeldService;
pub use weld_grpc_rust::*;

use tls_api::TlsConnector;
use tls_api::TlsConnectorBuilder;

use std::net::SocketAddr;
use std::sync::Arc;

#[derive(Clone)]
pub struct WeldServerClient {
    client: Arc<weld_grpc_rust::WeldServiceClient>,
    username: String,
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
}

impl WeldServerClient {
    pub fn new(hostname: &str, username: String, port: u16) -> Self {
        WeldServerClient {
            client: Arc::new(
                weld_grpc_rust::WeldServiceClient::new_plain(hostname, port, Default::default())
                    .unwrap(),
            ),
            username: username,
        }
    }

    pub fn new_tls(
        _hostname: &str,
        tls_hostname: &str,
        username: String,
        port: u16,
        root_ca: Vec<u8>,
        _client_key: Vec<u8>,
    ) -> Self {
        // Add the root certificate.
        let root_ca = tls_api::Certificate::from_der(root_ca);
        let mut builder = tls_api_native_tls::TlsConnector::builder().unwrap();
        builder
            .add_root_certificate(root_ca)
            .expect("add_root_certificate");

        let connector = Arc::new(builder.build().unwrap());
        let tls_option = httpbis::ClientTlsOption::Tls(tls_hostname.to_owned(), connector);

        let addr = SocketAddr::new("::1".parse().unwrap(), port);
        let grpc_client =
            grpc::Client::new_expl(&addr, tls_hostname, tls_option, Default::default()).unwrap();

        WeldServerClient {
            client: Arc::new(weld_grpc_rust::WeldServiceClient::with_client(grpc_client)),
            username: username,
        }
    }

    fn opts(&self) -> grpc::RequestOptions {
        grpc::RequestOptions::new()
    }
}

impl WeldServer for WeldServerClient {
    fn read(&self, req: weld::FileIdentifier) -> weld::File {
        self.client.read(self.opts(), req).wait().expect("rpc").1
    }

    fn read_attrs(&self, req: weld::FileIdentifier) -> weld::File {
        self.client
            .read_attrs(self.opts(), req)
            .wait()
            .expect("rpc")
            .1
    }

    fn submit(&self, req: weld::Change) -> weld::SubmitResponse {
        self.client.submit(self.opts(), req).wait().expect("rpc").1
    }

    fn snapshot(&self, req: weld::Change) -> weld::SnapshotResponse {
        self.client
            .snapshot(self.opts(), req)
            .wait()
            .expect("rpc")
            .1
    }

    fn get_change(&self, req: weld::Change) -> weld::Change {
        self.client
            .get_change(self.opts(), req)
            .wait()
            .expect("rpc")
            .1
    }

    fn list_changes(&self) -> Vec<Change> {
        let req = weld::ListChangesRequest::new();
        self.client
            .list_changes(self.opts(), req)
            .wait()
            .expect("rpc")
            .1
            .take_changes()
            .into_vec()
    }

    fn get_latest_change(&self) -> weld::Change {
        self.client
            .get_latest_change(self.opts(), weld::GetLatestChangeRequest::new())
            .wait()
            .expect("rpc")
            .1
    }

    fn list_files(&self, req: weld::FileIdentifier) -> Vec<File> {
        self.client
            .list_files(self.opts(), req)
            .wait()
            .expect("rpc")
            .1
            .take_files()
            .into_vec()
    }

    fn get_submitted_changes(&self, req: weld::GetSubmittedChangesRequest) -> Vec<Change> {
        self.client
            .get_submitted_changes(self.opts(), req)
            .wait()
            .expect("rpc")
            .1
            .take_changes()
            .into_vec()
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
        self.client
            .make_change(self.opts(), req)
            .wait()
            .expect("rpc")
            .1
    }

    pub fn read(&self, req: weld::FileIdentifier) -> weld::File {
        self.client.read(self.opts(), req).wait().expect("rpc").1
    }

    pub fn write(&self, req: weld::WriteRequest) {
        self.client.write(self.opts(), req).wait().expect("rpc");
    }

    pub fn delete(&self, req: weld::FileIdentifier) {
        self.client.delete(self.opts(), req).wait().expect("rpc");
    }

    pub fn get_change(&self, req: weld::GetChangeRequest) -> weld::Change {
        self.client
            .get_change(self.opts(), req)
            .wait()
            .expect("rpc")
            .1
    }

    pub fn list_files(&self, req: weld::FileIdentifier) -> Vec<File> {
        self.client
            .list_files(self.opts(), req)
            .wait()
            .expect("rpc")
            .1
            .take_files()
            .into_vec()
    }

    pub fn list_changes(&self) -> Vec<Change> {
        let req = weld::ListChangesRequest::new();
        self.client
            .list_changes(self.opts(), req)
            .wait()
            .expect("rpc")
            .1
            .take_changes()
            .into_vec()
    }

    pub fn snapshot(&self, req: weld::Change) -> weld::SnapshotResponse {
        self.client
            .snapshot(self.opts(), req)
            .wait()
            .expect("rpc")
            .1
    }

    pub fn submit(&self, req: weld::Change) -> weld::SubmitResponse {
        self.client.submit(self.opts(), req).wait().expect("rpc").1
    }

    pub fn lookup_friendly_name(&self, name: String) -> Option<u64> {
        let mut req = weld::LookupFriendlyNameRequest::new();
        req.set_friendly_name(name);
        match self
            .client
            .lookup_friendly_name(self.opts(), req)
            .wait()
            .expect("rpc")
            .1
            .get_id()
        {
            0 => None,
            x => Some(x),
        }
    }

    pub fn get_patch(&self, req: weld::Change) -> String {
        self.client
            .get_patch(self.opts(), req)
            .wait()
            .expect("rpc")
            .1
            .get_patch()
            .to_owned()
    }

    pub fn sync(&self, req: weld::SyncRequest) -> weld::SyncResponse {
        self.client.sync(self.opts(), req).wait().expect("rpc").1
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
    if filename.ends_with(".swp") {
        return true;
    }
    if filename.ends_with(".swo") {
        return true;
    }
    if filename.ends_with("~") {
        return true;
    }

    false
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
