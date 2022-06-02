use tls_api::TlsConnector;
use tls_api::TlsConnectorBuilder;

use std::net::ToSocketAddrs;
use std::sync::Arc;

pub fn make_tls_client(hostname: &str, port: u16) -> grpc::Client {
    let mut builder = tls_api_openssl::TlsConnector::builder().unwrap();
    builder.set_alpn_protocols(&[b"h2"]).unwrap();
    let connector = Arc::new(builder.build().unwrap());
    let tls_option = httpbis::ClientTlsOption::Tls(hostname.to_owned(), connector);
    grpc::ClientBuilder::new(hostname, port)
        .explicit_tls(tls_option)
        .build()
        .unwrap()
}
