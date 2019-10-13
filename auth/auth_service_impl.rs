pub struct AuthServiceHandler {}
impl AuthServiceHandler {
    pub fn new() -> Self {
        Self {}
    }
}

impl auth_grpc_rust::AuthenticationService for AuthServiceHandler {
    fn login(
        &self,
        _m: grpc::RequestOptions,
        req: auth_grpc_rust::LoginRequest,
    ) -> grpc::SingleResponse<auth_grpc_rust::LoginChallenge> {
        grpc::SingleResponse::completed(auth_grpc_rust::LoginChallenge::new())
    }

    fn authenticate(
        &self,
        _m: grpc::RequestOptions,
        req: auth_grpc_rust::AuthenticateRequest,
    ) -> grpc::SingleResponse<auth_grpc_rust::AuthenticateResponse> {
        grpc::SingleResponse::completed(auth_grpc_rust::AuthenticateResponse::new())
    }
}
