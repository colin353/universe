extern crate auth_client;
extern crate grpc;
extern crate largetable_client;
extern crate x20_grpc_rust as x20;

use auth_client::AuthServer;
use largetable_client::LargeTableClient;

const BINARY_VERSIONS: &'static str = "x20::binary_versions";
const BINARIES: &'static str = "x20::binaries";
const CONFIG_VERSIONS: &'static str = "x20::config_versions";
const CONFIGS: &'static str = "x20::configs";

#[derive(Clone)]
pub struct X20ServiceHandler<C: LargeTableClient> {
    database: C,
    auth: auth_client::AuthClient,
}

fn config_rowname(env: &str) -> String {
    format!("{}/{}", CONFIGS, env)
}

impl<C: LargeTableClient + Clone> X20ServiceHandler<C> {
    pub fn new(db: C, auth: auth_client::AuthClient) -> Self {
        Self {
            database: db,
            auth: auth,
        }
    }

    pub fn get_binaries(&self) -> x20::GetBinariesResponse {
        let bin_iter = largetable_client::LargeTableScopedIterator::new(
            &self.database,
            String::from(BINARIES),
            String::from(""),
            String::from(""),
            String::from(""),
            0,
        );
        let mut response = x20::GetBinariesResponse::new();
        for (_, bin) in bin_iter {
            response.mut_binaries().push(bin);
        }
        response
    }

    fn authenticate(&self, token: &str) -> bool {
        self.auth.authenticate(token.to_owned()).get_success()
    }

    pub fn publish_binary(
        &self,
        mut req: x20::PublishBinaryRequest,
        require_auth: bool,
    ) -> x20::PublishBinaryResponse {
        if require_auth && !self.authenticate(req.get_token()) {
            let mut response = x20::PublishBinaryResponse::new();
            response.set_error(x20::Error::AUTHENTICATION);
            return response;
        }

        let name = req.get_binary().get_name().to_owned();

        // If deletion, delete the binary
        if req.get_delete() {
            self.database.delete(BINARIES, &name);
            return x20::PublishBinaryResponse::new();
        }

        if req.get_binary().get_name().is_empty() {
            eprintln!("cannot publish empty binary name");
            return x20::PublishBinaryResponse::new();
        }
        let version = self
            .database
            .reserve_id(BINARY_VERSIONS, req.get_binary().get_name());

        let mut binary = req.take_binary();
        binary.set_version(version);

        self.database.write_proto(BINARIES, &name, 0, &binary);

        x20::PublishBinaryResponse::new()
    }

    pub fn get_configs(&self, req: x20::GetConfigsRequest) -> x20::GetConfigsResponse {
        let configs_iter = largetable_client::LargeTableScopedIterator::new(
            &self.database,
            config_rowname(req.get_environment()),
            String::from(""),
            String::from(""),
            String::from(""),
            0,
        );
        let mut response = x20::GetConfigsResponse::new();
        for (_, config) in configs_iter {
            response.mut_configs().push(config);
        }
        response
    }

    pub fn publish_config(&self, mut req: x20::PublishConfigRequest) -> x20::PublishConfigResponse {
        if !self.authenticate(req.get_token()) {
            let mut response = x20::PublishConfigResponse::new();
            response.set_error(x20::Error::AUTHENTICATION);
            return response;
        }

        let name = req.get_config().get_name().to_owned();

        // If deletion, delete the config
        if req.get_delete() {
            self.database
                .delete(&config_rowname(req.get_config().get_environment()), &name);
            return x20::PublishConfigResponse::new();
        }

        if name.is_empty() {
            eprintln!("cannot publish empty config name");
            return x20::PublishConfigResponse::new();
        }
        let version = self.database.reserve_id(CONFIG_VERSIONS, &name);

        let mut config = req.take_config();
        config.set_version(version);

        self.database
            .write_proto(&config_rowname(config.get_environment()), &name, 0, &config);

        x20::PublishConfigResponse::new()
    }
}

impl<C: LargeTableClient + Clone> x20::X20Service for X20ServiceHandler<C> {
    fn get_binaries(
        &self,
        _: grpc::ServerHandlerContext,
        req: grpc::ServerRequestSingle<x20::GetBinariesRequest>,
        resp: grpc::ServerResponseUnarySink<x20::GetBinariesResponse>,
    ) -> grpc::Result<()> {
        resp.finish(self.get_binaries())
    }

    fn publish_binary(
        &self,
        _: grpc::ServerHandlerContext,
        req: grpc::ServerRequestSingle<x20::PublishBinaryRequest>,
        resp: grpc::ServerResponseUnarySink<x20::PublishBinaryResponse>,
    ) -> grpc::Result<()> {
        resp.finish(self.publish_binary(req.message, true))
    }

    fn get_configs(
        &self,
        _: grpc::ServerHandlerContext,
        req: grpc::ServerRequestSingle<x20::GetConfigsRequest>,
        resp: grpc::ServerResponseUnarySink<x20::GetConfigsResponse>,
    ) -> grpc::Result<()> {
        resp.finish(self.get_configs(req.message))
    }

    fn publish_config(
        &self,
        _: grpc::ServerHandlerContext,
        req: grpc::ServerRequestSingle<x20::PublishConfigRequest>,
        resp: grpc::ServerResponseUnarySink<x20::PublishConfigResponse>,
    ) -> grpc::Result<()> {
        resp.finish(self.publish_config(req.message))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate largetable_test;

    fn create_test_handler() -> X20ServiceHandler<largetable_test::LargeTableMockClient> {
        let db = largetable_test::LargeTableMockClient::new();
        X20ServiceHandler::new(db, auth_client::AuthClient::new_fake())
    }

    #[test]
    fn test_publish() {
        let handler = create_test_handler();
        let mut req = x20::PublishBinaryRequest::new();
        req.mut_binary().set_name(String::from("vim"));
        req.mut_binary().set_url(String::from("http://google.com"));
        req.mut_binary().set_target(String::from("//vim:vim"));

        handler.publish_binary(req, true);

        // Should be able to read that back
        let response = handler.get_binaries();
        assert_eq!(response.get_binaries().len(), 1);
        assert_eq!(response.get_binaries()[0].get_name(), "vim");
    }

    #[test]
    fn test_publish_config() {
        let handler = create_test_handler();
        let mut req = x20::PublishConfigRequest::new();
        req.mut_config().set_name(String::from("vim"));
        req.mut_config().set_environment(String::from("desktop"));

        handler.publish_config(req);

        // Should be able to read that back
        let mut req = x20::GetConfigsRequest::new();
        req.set_environment(String::from("desktop"));
        let response = handler.get_configs(req);
        assert_eq!(response.get_configs().len(), 1);
        assert_eq!(response.get_configs()[0].get_name(), "vim");
    }
}
