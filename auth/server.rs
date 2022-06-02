#[macro_use]
extern crate flags;

use std::collections::HashMap;
use std::iter::FromIterator;
use std::sync::{Arc, RwLock};

#[tokio::main]
async fn main() {
    let grpc_port = define_flag!("grpc_port", 8888, "The gRPC port to bind to.");
    let web_port = define_flag!("web_port", 8899, "The web port to bind to.");
    let oauth_client_id = define_flag!("oauth_client_id", String::new(), "The oauth client ID");
    let oauth_client_secret = define_flag!(
        "oauth_client_secret",
        String::new(),
        "The oauth client secret"
    );
    let hostname = define_flag!(
        "hostname",
        String::new(),
        "the publicly accessible hostname"
    );
    let allowed_emails = define_flag!(
        "allowed_emails",
        String::new(),
        "a list of allowed emails and usernames separated by colon and commas (example: colin:colin353@gmail.com,tester:tester@test.com)"
    );
    let cookie_domain = define_flag!(
        "cookie_domain",
        String::from("colinmerkel.xyz"),
        "the domain setting to use for cookies"
    );
    let gcp_token_location = define_flag!(
        "gcp_token_location",
        String::from("/gcp-access.json"),
        "the location of the gcp access json file"
    );
    let secret_key = define_flag!("secret_key", String::new(), "the shared secret key string");

    parse_flags!(
        allowed_emails,
        grpc_port,
        web_port,
        hostname,
        cookie_domain,
        oauth_client_id,
        oauth_client_secret,
        secret_key,
        gcp_token_location
    );

    let email_whitelist =
        std::collections::HashMap::from_iter(allowed_emails.value().split(",").map(|x| {
            let components: Vec<_> = x.split(":").collect();
            if components.len() == 1 {
                return (components[0].to_owned(), components[0].to_owned());
            }
            (components[1].to_owned(), components[0].to_owned())
        }));

    let mut server = grpc::ServerBuilder::<tls_api_openssl::TlsAcceptor>::new();
    server.http.set_port(grpc_port.value());

    let default_access_token =
        std::fs::read_to_string(gcp_token_location.value()).unwrap_or_default();

    let tokens = Arc::new(RwLock::new(HashMap::new()));
    let handler = auth_service_impl::AuthServiceHandler::new(
        hostname.value(),
        oauth_client_id.value(),
        tokens.clone(),
        secret_key.value(),
        default_access_token,
    );

    server.add_service(auth_grpc_rust::AuthenticationServiceServer::new_service_def(handler));

    let _server = server.build().expect("server");
    ws::serve(
        auth_service_impl::AuthWebServer::new(
            tokens,
            hostname.value(),
            cookie_domain.value(),
            oauth_client_id.value(),
            oauth_client_secret.value(),
            Arc::new(email_whitelist),
        ),
        web_port.value(),
    )
    .await;
}
