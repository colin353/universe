use largetable_client::LargeTableClient;
use protobuf;
use std::collections::HashSet;
use std::io::Write;
use weld;
use weld::WeldServer;

#[derive(Clone)]
pub struct WeldLocalServiceHandler<C: LargeTableClient> {
    repo: weld_repo::Repo<C, weld::WeldServerClient>,
    mount_dir: String,
}

impl<C: LargeTableClient> WeldLocalServiceHandler<C> {
    pub fn new(repo: weld_repo::Repo<C, weld::WeldServerClient>) -> Self {
        Self {
            repo: repo,
            mount_dir: String::new(),
        }
    }

    pub fn set_mount_dir(&mut self, mount_dir: String) {
        self.mount_dir = mount_dir;
    }

    pub fn get_change(&self, change: weld::GetChangeRequest) -> weld::Change {
        match self.repo.get_change(change.get_change().get_id()) {
            Some(mut c) => {
                // Fill the change with staged file changes
                if change.get_filled() {
                    self.repo.fill_change(&mut c);
                }
                c
            }
            None => weld::Change::new(),
        }
    }

    pub fn make_change(&self, change: weld::Change) -> weld::Change {
        let id = self.repo.make_change(change);
        self.repo.get_change(id).unwrap()
    }

    pub fn read(&self, ident: weld::FileIdentifier) -> weld::File {
        match self
            .repo
            .read(ident.get_id(), ident.get_filename(), ident.get_index())
        {
            Some(f) => f,
            None => weld::File::new(),
        }
    }

    pub fn write(&self, req: weld::WriteRequest) -> weld::WriteResponse {
        self.repo.write(req.get_id(), req.get_file().clone(), 0);
        weld::WriteResponse::new()
    }

    pub fn delete(&self, ident: weld::FileIdentifier) -> weld::DeleteResponse {
        self.repo
            .delete(ident.get_id(), ident.get_filename(), ident.get_id());
        weld::DeleteResponse::new()
    }

    pub fn list_files(&self, ident: weld::FileIdentifier) -> weld::ListFilesResponse {
        let mut response = weld::ListFilesResponse::new();
        response.set_files(protobuf::RepeatedField::from_vec(self.repo.list_files(
            ident.get_id(),
            ident.get_filename(),
            0,
        )));
        response
    }

    pub fn list_changes(&self) -> weld::ListChangesResponse {
        let mut response = weld::ListChangesResponse::new();
        response.set_changes(protobuf::RepeatedField::from_vec(
            self.repo.list_changes().collect(),
        ));
        response
    }

    pub fn snapshot(&self, change: weld::Change) -> weld::SnapshotResponse {
        match change.get_id() {
            0 => match self.repo.lookup_friendly_name(change.get_friendly_name()) {
                Some(x) => x,
                None => return weld::SnapshotResponse::new(),
            },
            x => x,
        };
        self.repo.snapshot(&change)
    }

    pub fn submit(&self, change: weld::Change) -> weld::SubmitResponse {
        let id = match change.get_id() {
            0 => match self.repo.lookup_friendly_name(change.get_friendly_name()) {
                Some(x) => x,
                None => return weld::SubmitResponse::new(),
            },
            x => x,
        };
        self.repo.submit(id)
    }

    pub fn get_patch(&self, change: weld::Change) -> weld::Patch {
        let mut req = weld::GetChangeRequest::new();
        req.set_change(change);
        req.set_filled(true);
        let change = self.get_change(req);
        let mut patch = weld::Patch::new();
        patch.set_patch(self.repo.patch(&change));
        patch
    }

    pub fn sync(&self, req: &weld::SyncRequest) -> weld::SyncResponse {
        let mut change_req = weld::GetChangeRequest::new();
        change_req.set_change(req.get_change().clone());
        change_req.set_filled(true);
        let change = self.get_change(change_req);
        let (conflicted_files, synced_index) =
            self.repo.sync(change.get_id(), req.get_conflicted_files());

        let mut response = weld::SyncResponse::new();
        response.set_conflicted_files(protobuf::RepeatedField::from_vec(conflicted_files));
        response.set_index(synced_index);
        response
    }

    pub fn run_build(&self, req: &weld::RunBuildRequest) -> weld::RunBuildResponse {
        let mut response = weld::RunBuildResponse::new();

        let output = match std::process::Command::new("bazel")
            .arg("build")
            .arg(req.get_target())
            .current_dir(format!(
                "{}/unsubmitted/{}",
                self.mount_dir,
                req.get_change_id()
            ))
            .output()
        {
            Ok(o) => o,
            Err(e) => {
                println!("build command failed to start: {:?}", e);
                return response;
            }
        };

        let build_stdout = std::str::from_utf8(&output.stdout)
            .unwrap()
            .trim()
            .to_owned();
        let build_stderr = std::str::from_utf8(&output.stderr)
            .unwrap()
            .trim()
            .to_owned();
        response.set_build_output(format!("{}\n{}", build_stdout, build_stderr));
        if output.status.success() {
            response.set_build_success(true);
        } else {
            println!("command failed: {}", response.get_build_output());
            return response;
        }

        let output = match std::process::Command::new("bazel")
            .arg("test")
            .arg("--test_output=errors")
            .arg(req.get_target())
            .current_dir(format!(
                "{}/unsubmitted/{}",
                self.mount_dir,
                req.get_change_id()
            ))
            .output()
        {
            Ok(o) => o,
            Err(_) => {
                response.set_build_output(String::from("could not start test command!"));
                return response;
            }
        };

        let test_stdout = std::str::from_utf8(&output.stdout)
            .unwrap()
            .trim()
            .to_owned();
        let test_stderr = std::str::from_utf8(&output.stderr)
            .unwrap()
            .trim()
            .to_owned();
        response.set_test_output(format!("{}\n{}", test_stdout, test_stderr));

        // Status code 4 means that there were no errors, but no test targets
        if output.status.success() || output.status.code() == Some(4) {
            response.set_test_success(true);
        } else {
            println!("command failed: {}", response.get_test_output());
            return response;
        }

        response.set_success(true);
        response
    }

    pub fn run_build_query(&self, req: &weld::RunBuildQueryRequest) -> weld::RunBuildQueryResponse {
        let remote_server = match self.repo.remote_server {
            Some(ref r) => r,
            None => {
                println!("no remote server to connect to");
                return weld::RunBuildQueryResponse::new();
            }
        };

        let mut c = weld::Change::new();
        c.set_id(req.get_change_id());
        let change = remote_server.get_change(c);

        if !change.get_found() {
            println!("change not found");
            return weld::RunBuildQueryResponse::new();
        }

        let maybe_last_snapshot = change
            .get_changes()
            .iter()
            .filter_map(|c| c.get_snapshots().iter().map(|x| x.get_snapshot_id()).max())
            .max();

        let last_snapshot_id = match maybe_last_snapshot {
            Some(x) => x,
            None => {
                println!("change contains no changes");
                return weld::RunBuildQueryResponse::new();
            }
        };

        let changes = change
            .get_changes()
            .iter()
            .filter_map(|h| {
                h.get_snapshots()
                    .iter()
                    .filter(|x| x.get_snapshot_id() == last_snapshot_id)
                    .next()
            })
            .filter(|f| !f.get_directory() && !f.get_reverted())
            .map(|f| f.get_filename()[1..].to_owned())
            .collect::<Vec<_>>();

        println!("found changed files: {:?}", changes);

        let mut files = HashSet::new();
        for changed_file in &changes {
            let output = match std::process::Command::new("bazel")
                .arg("query")
                .arg(changed_file)
                .current_dir(format!(
                    "{}/unsubmitted/{}",
                    self.mount_dir,
                    req.get_change_id()
                ))
                .output()
            {
                Ok(o) => o,
                Err(e) => {
                    println!("command failed to start: {:?}", e);
                    return weld::RunBuildQueryResponse::new();
                }
            };

            let file = std::str::from_utf8(&output.stdout)
                .unwrap()
                .trim()
                .to_owned();
            if output.status.success() && !file.is_empty() {
                files.insert(file);
            } else {
                let errors = std::str::from_utf8(&output.stderr)
                    .unwrap()
                    .trim()
                    .to_owned();
                println!("file query failed: {}", errors);
            }
        }

        println!("files: {:?}", files);

        let mut targets = HashSet::new();
        for file in &files {
            let output = match std::process::Command::new("bazel")
                .arg("query")
                .arg(format!("attr('srcs', {}, //...)", file))
                .current_dir(format!(
                    "{}/unsubmitted/{}",
                    self.mount_dir,
                    req.get_change_id()
                ))
                .output()
            {
                Ok(o) => o,
                Err(e) => {
                    println!("command failed to start: {:?}", e);
                    return weld::RunBuildQueryResponse::new();
                }
            };

            let targets_output = std::str::from_utf8(&output.stdout)
                .unwrap()
                .trim()
                .to_owned();
            if output.status.success() && !targets_output.is_empty() {
                for target in targets_output.lines() {
                    targets.insert(target.trim().to_owned());
                }
            } else {
                let errors = std::str::from_utf8(&output.stderr)
                    .unwrap()
                    .trim()
                    .to_owned();
                println!("target query failed: {}", errors);
            }
        }

        let mut dependencies = HashSet::new();
        for target in &targets {
            let output = match std::process::Command::new("bazel")
                .arg("query")
                .arg(format!("rdeps(//..., {})", target))
                .current_dir(format!(
                    "{}/unsubmitted/{}",
                    self.mount_dir,
                    req.get_change_id()
                ))
                .output()
            {
                Ok(o) => o,
                Err(e) => {
                    println!("command failed to start: {:?}", e);
                    return weld::RunBuildQueryResponse::new();
                }
            };

            let dependencies_output = std::str::from_utf8(&output.stdout)
                .unwrap()
                .trim()
                .to_owned();
            if output.status.success() && !dependencies_output.is_empty() {
                for dependency in dependencies_output.lines() {
                    dependencies.insert(dependency.trim().to_owned());
                }
            } else {
                let errors = std::str::from_utf8(&output.stderr)
                    .unwrap()
                    .trim()
                    .to_owned();
                println!("dependency query failed: {}", errors);
            }
        }

        let mut response = weld::RunBuildQueryResponse::new();
        response.set_success(true);
        for target in &targets {
            response.mut_targets().push(target.to_owned());
        }
        for dependency in dependencies {
            if !targets.contains(dependency.as_str()) {
                response.mut_dependencies().push(dependency.to_owned());
            }
        }
        response
    }

    pub fn apply_patch(&self, req: weld::ApplyPatchRequest) -> weld::ApplyPatchResponse {
        let mut response = weld::ApplyPatchResponse::new();

        let remote_server = match self.repo.remote_server {
            Some(ref r) => r,
            None => {
                println!("no remote server to connect to");
                response.set_reason(String::from("no remote server to connect to"));
                return response;
            }
        };

        let mut c = weld::Change::new();
        c.set_id(req.get_change_id());
        let mut change = remote_server.get_change(c);

        if !change.get_found() {
            println!("change not found");
            response.set_reason(String::from("change not found"));
            return response;
        }

        let maybe_last_snapshot = change
            .get_changes()
            .iter()
            .filter_map(|c| c.get_snapshots().iter().map(|x| x.get_snapshot_id()).max())
            .max();

        let last_snapshot_id = match maybe_last_snapshot {
            Some(x) => x,
            None => {
                println!("change contains no changes");
                return weld::ApplyPatchResponse::new();
            }
        };

        let changes = change
            .get_changes()
            .iter()
            .filter_map(|h| {
                h.get_snapshots()
                    .iter()
                    .filter(|x| x.get_snapshot_id() == last_snapshot_id)
                    .next()
            })
            .filter(|f| !f.get_directory() && !f.get_reverted())
            .map(|f| f.to_owned())
            .collect::<Vec<_>>();

        change.set_staged_files(protobuf::RepeatedField::from_vec(changes));
        change.set_is_based_locally(false);
        let patch = self.repo.patch(&change);
        let mut f = match std::fs::File::create("/tmp/patch.txt") {
            Ok(f) => f,
            Err(e) => {
                println!("could not create /tmp/patch.txt: {:?}", e);
                response.set_reason(String::from("could not create /tmp/patch.txt"));
                return response;
            }
        };
        match f.write_all(patch.as_bytes()) {
            Ok(_) => (),
            Err(e) => {
                println!("failed to write patch file: {:?}", e);
                response.set_reason(String::from("failed to write patch file"));
                return response;
            }
        };

        // If the /tmp/github directory doesn't exist, and the repository isn't
        // checked out yet, create the directory and check out the repo
        if !std::path::Path::new("/tmp/github").exists() {
            match std::fs::create_dir("/tmp/github") {
                Ok(_) => (),
                Err(e) => {
                    println!("failed to create /tmp/github directory: {:?}", e);
                    response.set_reason(String::from("failed to create /tmp/github directory"));
                    return response;
                }
            };
            let output = match std::process::Command::new("git")
                .arg("clone")
                .arg("git@github.com:colin353/universe.git")
                .current_dir("/tmp/github")
                .output()
            {
                Ok(o) => o,
                Err(e) => {
                    println!("clone command failed to start: {:?}", e);
                    response.set_reason(String::from("clone command failed to start"));
                    return response;
                }
            };
            if !output.status.success() {
                println!("failed to clone repo");
                let err_msg = std::str::from_utf8(&output.stderr)
                    .unwrap()
                    .trim()
                    .to_owned();
                response.set_reason(format!("failed to clone repo: {:?}", err_msg));
                return response;
            }
        }

        // Pull the branch to make sure it's up to date
        let output = match std::process::Command::new("git")
            .arg("pull")
            .current_dir("/tmp/github/universe")
            .output()
        {
            Ok(o) => o,
            Err(e) => {
                println!("pull command failed to start: {:?}", e);
                response.set_reason(String::from("pull command failed to start"));
                return response;
            }
        };
        if !output.status.success() {
            println!("failed to pull repo");
            let err_msg = std::str::from_utf8(&output.stderr)
                .unwrap()
                .trim()
                .to_owned();
            response.set_reason(format!("failed to pull repo: {:?}", err_msg));
            return response;
        }

        // Apply the patch
        let output = match std::process::Command::new("git")
            .arg("apply")
            .arg("/tmp/patch.txt")
            .current_dir("/tmp/github/universe")
            .output()
        {
            Ok(o) => o,
            Err(e) => {
                println!("patch command failed to start: {:?}", e);
                response.set_reason(String::from("patch command failed to start"));
                return response;
            }
        };
        if !output.status.success() {
            println!("failed to patch repo");
            let err_msg = std::str::from_utf8(&output.stderr)
                .unwrap()
                .trim()
                .to_owned();
            response.set_reason(format!("failed to patch repo: {:?}", err_msg));
            return response;
        }

        // Stage the patch changes
        let output = match std::process::Command::new("git")
            .arg("add")
            .arg(".")
            .current_dir("/tmp/github/universe")
            .output()
        {
            Ok(o) => o,
            Err(e) => {
                println!("staging command failed to start: {:?}", e);
                response.set_reason(String::from("staging command failed to start"));
                return response;
            }
        };
        if !output.status.success() {
            println!("failed to stage change");
            let err_msg = std::str::from_utf8(&output.stderr)
                .unwrap()
                .trim()
                .to_owned();
            response.set_reason(format!("failed to stage change: {:?}", err_msg));
            return response;
        }

        // Commit the patch changes
        let output = match std::process::Command::new("git")
            .arg("commit")
            .arg("-m")
            .arg(change.get_friendly_name())
            .current_dir("/tmp/github/universe")
            .output()
        {
            Ok(o) => o,
            Err(e) => {
                println!("commit command failed to start: {:?}", e);
                response.set_reason(format!("failed to commit change: {:?}", e));
                return response;
            }
        };
        if !output.status.success() {
            println!("failed to commit change");
            let err_msg = std::str::from_utf8(&output.stderr)
                .unwrap()
                .trim()
                .to_owned();
            response.set_reason(format!("failed to commit change: {:?}", err_msg));
            return response;
        }

        // Push the committed changes
        let output = match std::process::Command::new("git")
            .arg("push")
            .current_dir("/tmp/github/universe")
            .output()
        {
            Ok(o) => o,
            Err(e) => {
                println!("push command failed to start: {:?}", e);
                response.set_reason(format!("push command failed to start: {:?}", e));
                return response;
            }
        };
        if !output.status.success() {
            println!("failed to push change");
            let err_msg = std::str::from_utf8(&output.stderr)
                .unwrap()
                .trim()
                .to_owned();
            response.set_reason(format!("failed to push change: {:?}", err_msg));
            return response;
        }

        response.set_success(true);
        response
    }

    fn delete_change(&self, change: &weld::Change) -> weld::DeleteResponse {
        self.repo.delete_change(change.get_id());
        weld::DeleteResponse::new()
    }
}

impl<C: LargeTableClient> weld::WeldLocalService for WeldLocalServiceHandler<C> {
    fn get_change(
        &self,
        _m: grpc::RequestOptions,
        req: weld::GetChangeRequest,
    ) -> grpc::SingleResponse<weld::Change> {
        grpc::SingleResponse::completed(self.get_change(req))
    }

    fn make_change(
        &self,
        _m: grpc::RequestOptions,
        req: weld::Change,
    ) -> grpc::SingleResponse<weld::Change> {
        grpc::SingleResponse::completed(self.make_change(req))
    }

    fn read(
        &self,
        _m: grpc::RequestOptions,
        req: weld::FileIdentifier,
    ) -> grpc::SingleResponse<weld::File> {
        grpc::SingleResponse::completed(self.read(req))
    }

    fn write(
        &self,
        _m: grpc::RequestOptions,
        req: weld::WriteRequest,
    ) -> grpc::SingleResponse<weld::WriteResponse> {
        grpc::SingleResponse::completed(self.write(req))
    }

    fn list_files(
        &self,
        _m: grpc::RequestOptions,
        req: weld::FileIdentifier,
    ) -> grpc::SingleResponse<weld::ListFilesResponse> {
        grpc::SingleResponse::completed(self.list_files(req))
    }

    fn delete(
        &self,
        _m: grpc::RequestOptions,
        req: weld::FileIdentifier,
    ) -> grpc::SingleResponse<weld::DeleteResponse> {
        grpc::SingleResponse::completed(self.delete(req))
    }

    fn list_changes(
        &self,
        _m: grpc::RequestOptions,
        _req: weld::ListChangesRequest,
    ) -> grpc::SingleResponse<weld::ListChangesResponse> {
        grpc::SingleResponse::completed(self.list_changes())
    }

    fn snapshot(
        &self,
        _m: grpc::RequestOptions,
        req: weld::Change,
    ) -> grpc::SingleResponse<weld::SnapshotResponse> {
        grpc::SingleResponse::completed(self.snapshot(req))
    }

    fn submit(
        &self,
        _m: grpc::RequestOptions,
        req: weld::Change,
    ) -> grpc::SingleResponse<weld::SubmitResponse> {
        grpc::SingleResponse::completed(self.submit(req))
    }

    fn lookup_friendly_name(
        &self,
        _m: grpc::RequestOptions,
        req: weld::LookupFriendlyNameRequest,
    ) -> grpc::SingleResponse<weld::LookupFriendlyNameResponse> {
        let id = match self.repo.lookup_friendly_name(req.get_friendly_name()) {
            Some(id) => id,
            None => 0,
        };

        let mut response = weld::LookupFriendlyNameResponse::new();
        response.set_id(id);
        grpc::SingleResponse::completed(response)
    }

    fn get_patch(
        &self,
        _m: grpc::RequestOptions,
        req: weld::Change,
    ) -> grpc::SingleResponse<weld::Patch> {
        grpc::SingleResponse::completed(self.get_patch(req))
    }

    fn sync(
        &self,
        _m: grpc::RequestOptions,
        req: weld::SyncRequest,
    ) -> grpc::SingleResponse<weld::SyncResponse> {
        grpc::SingleResponse::completed(self.sync(&req))
    }

    fn run_build(
        &self,
        _m: grpc::RequestOptions,
        req: weld::RunBuildRequest,
    ) -> grpc::SingleResponse<weld::RunBuildResponse> {
        grpc::SingleResponse::completed(self.run_build(&req))
    }

    fn run_build_query(
        &self,
        _m: grpc::RequestOptions,
        req: weld::RunBuildQueryRequest,
    ) -> grpc::SingleResponse<weld::RunBuildQueryResponse> {
        grpc::SingleResponse::completed(self.run_build_query(&req))
    }

    fn apply_patch(
        &self,
        _m: grpc::RequestOptions,
        req: weld::ApplyPatchRequest,
    ) -> grpc::SingleResponse<weld::ApplyPatchResponse> {
        grpc::SingleResponse::completed(self.apply_patch(req))
    }

    fn delete_change(
        &self,
        _m: grpc::RequestOptions,
        req: weld::Change,
    ) -> grpc::SingleResponse<weld::DeleteResponse> {
        grpc::SingleResponse::completed(self.delete_change(&req))
    }
}
