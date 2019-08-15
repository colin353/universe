use largetable_client::LargeTableClient;
use weld;

#[derive(Clone)]
pub struct WeldLocalServiceHandler<C: LargeTableClient> {
    repo: weld_repo::Repo<C, weld::WeldServerClient>,
}

impl<C: LargeTableClient> WeldLocalServiceHandler<C> {
    pub fn new(repo: weld_repo::Repo<C, weld::WeldServerClient>) -> Self {
        Self { repo: repo }
    }

    pub fn get_change(&self, change: weld::Change) -> weld::Change {
        match self.repo.get_change(change.get_id()) {
            Some(mut c) => {
                // Fill the change with staged file changes
                self.repo.fill_change(&mut c);
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
        let change = self.get_change(change);
        let mut patch = weld::Patch::new();
        patch.set_patch(self.repo.patch(&change));
        patch
    }

    pub fn sync(&self, req: &weld::SyncRequest) -> weld::SyncResponse {
        let change = self.get_change(req.get_change().clone());
        let conflicted_files = self.repo.sync(change.get_id(), req.get_conflicted_files());

        let mut response = weld::SyncResponse::new();
        response.set_conflicted_files(protobuf::RepeatedField::from_vec(conflicted_files));
        response
    }
}

impl<C: LargeTableClient> weld::WeldLocalService for WeldLocalServiceHandler<C> {
    fn get_change(
        &self,
        _m: grpc::RequestOptions,
        req: weld::Change,
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
        req: weld::ListChangesRequest,
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
}
