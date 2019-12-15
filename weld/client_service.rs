use largetable_client::LargeTableClient;
use std::collections::HashSet;
use weld;

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

        let friendly_name = match self.repo.get_change(req.get_change_id()) {
            Some(x) => x.get_friendly_name().to_owned(),
            None => {
                println!("no such change: {}", req.get_change_id());
                response.set_build_output(String::from("could not find change to build"));
                return response;
            }
        };

        let output = match std::process::Command::new("bazel")
            .arg("build")
            .arg(req.get_target())
            .current_dir(format!("{}/local/{}", self.mount_dir, friendly_name))
            .output()
        {
            Ok(o) => o,
            Err(_) => {
                response.set_build_output(String::from("could not start build command!"));
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
            .arg(req.get_target())
            .current_dir(format!("{}/local/{}", self.mount_dir, friendly_name))
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
            response.set_build_success(true);
        } else {
            println!("command failed: {}", response.get_test_output());
            return response;
        }

        response.set_success(true);
        response
    }

    pub fn run_build_query(&self, req: &weld::RunBuildQueryRequest) -> weld::RunBuildQueryResponse {
        let friendly_name = match self.repo.get_change(req.get_change_id()) {
            Some(x) => x.get_friendly_name().to_owned(),
            None => {
                println!("no such change: {}", req.get_change_id());
                return weld::RunBuildQueryResponse::new();
            }
        };

        let changes = self
            .repo
            .list_changed_files(req.get_change_id(), 0)
            .filter(|f| !f.get_directory())
            .map(|f| f.get_filename()[1..].to_owned())
            .collect::<Vec<_>>();

        let mut files = HashSet::new();
        for changed_file in &changes {
            let output = match std::process::Command::new("bazel")
                .arg("query")
                .arg(changed_file)
                .current_dir(format!("{}/local/{}", self.mount_dir, friendly_name))
                .output()
            {
                Ok(o) => o,
                Err(_) => {
                    println!("command failed to start");
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
                .current_dir(format!("{}/local/{}", self.mount_dir, friendly_name))
                .output()
            {
                Ok(o) => o,
                Err(_) => {
                    println!("command failed to start");
                    return weld::RunBuildQueryResponse::new();
                }
            };

            let target = std::str::from_utf8(&output.stdout)
                .unwrap()
                .trim()
                .to_owned();
            if output.status.success() && !target.is_empty() {
                targets.insert(target);
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
                .current_dir(format!("{}/local/{}", self.mount_dir, friendly_name))
                .output()
            {
                Ok(o) => o,
                Err(_) => {
                    println!("command failed to start");
                    return weld::RunBuildQueryResponse::new();
                }
            };

            let dependency = std::str::from_utf8(&output.stdout)
                .unwrap()
                .trim()
                .to_owned();
            if output.status.success() && !dependency.is_empty() {
                dependencies.insert(target);
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
}
