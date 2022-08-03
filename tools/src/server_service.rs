use service::*;

pub struct SrcServer {
    table: managed_largetable::ManagedLargeTable,
}

impl SrcServer {
    pub fn new(root: std::path::PathBuf) -> std::io::Result<Self> {
        Ok(Self {
            table: managed_largetable::ManagedLargeTable::new(root)?,
        })
    }

    pub fn auth(&self, _token: &str) -> Result<String, String> {
        Ok(String::from("colin"))
    }
}

impl service::SrcServerServiceHandler for SrcServer {
    fn create(&self, req: CreateRequest) -> Result<CreateResponse, bus::BusRpcError> {
        let user = match self.auth(&req.token) {
            Ok(u) => u,
            Err(e) => {
                return Ok(CreateResponse {
                    failed: true,
                    error_message: e,
                })
            }
        };

        // TODO: validate that the name is OK
        if req.name.is_empty() {
            return Ok(CreateResponse {
                failed: true,
                error_message: String::from("must provide a valid repository name"),
            });
        }

        self.table
            .write(
                "repos".to_string(),
                format!("{}/{}", user, req.name),
                0,
                service::Repository {
                    host: String::new(),
                    owner: user,
                    name: req.name,
                    alias: String::new(),
                },
            )
            .map_err(|e| {
                eprintln!("{:?}", e);
                bus::BusRpcError::InternalError("failed to write repo".to_string())
            })?;

        Ok(CreateResponse {
            failed: false,
            ..Default::default()
        })
    }

    fn get_repository(
        &self,
        req: GetRepositoryRequest,
    ) -> Result<GetRepositoryResponse, bus::BusRpcError> {
        match self.auth(&req.token) {
            Ok(_) => (),
            Err(e) => {
                return Ok(GetRepositoryResponse {
                    failed: true,
                    error_message: e,
                    ..Default::default()
                })
            }
        };

        if let None =
            self.table
                .read::<bus::Nothing>("repos", &format!("{}/{}", req.owner, req.name), 0)
        {
            return Ok(GetRepositoryResponse {
                failed: true,
                error_message: "that repository doesn't exist".to_string(),
                ..Default::default()
            });
        }

        Ok(GetRepositoryResponse {
            failed: false,
            index: 1,
            ..Default::default()
        })
    }

    fn submit(&self, _: SubmitRequest) -> Result<SubmitResponse, bus::BusRpcError> {
        todo!()
    }

    fn get_blobs(&self, _: GetBlobsRequest) -> Result<GetBlobsResponse, bus::BusRpcError> {
        todo!()
    }
    fn get_metadata(&self, _: GetMetadataRequest) -> Result<GetMetadataRequest, bus::BusRpcError> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> SrcServer {
        let path = std::path::PathBuf::from("/tmp/asdf");
        std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path);
        SrcServer::new(path).unwrap()
    }

    #[test]
    fn test_create_repo() {
        let s = setup();
        let resp = s
            .get_repository(GetRepositoryRequest {
                token: String::new(),
                owner: "colin".to_string(),
                name: "universe".to_string(),
            })
            .unwrap();
        assert_eq!(resp.failed, true);

        let resp = s
            .create(CreateRequest {
                token: String::new(),
                name: "universe".to_string(),
            })
            .unwrap();
        assert_eq!(resp.failed, false);

        let resp = s
            .get_repository(GetRepositoryRequest {
                token: String::new(),
                owner: "colin".to_string(),
                name: "universe".to_string(),
            })
            .unwrap();
        assert_eq!(resp.failed, false);
        assert_eq!(resp.index, 1);
    }
}
