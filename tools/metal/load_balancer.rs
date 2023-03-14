use hyper::http;
use hyper::service::{make_service_fn, service_fn};
use hyper::upgrade::Upgraded;
use hyper::{Body, Client, Method, Request, Response, Server};
use std::convert::Infallible;
use std::net::SocketAddr;
use tokio::net::TcpStream;

type HttpClient = Client<hyper::client::HttpConnector>;

pub trait Resolver: Send + Sync + 'static {
    fn resolve(&self, host: &str) -> Option<(std::net::IpAddr, u16)>;
}

pub async fn proxy(port: u16, resolver: std::sync::Arc<dyn Resolver>) {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let client = HttpClient::new();
    let make_service = make_service_fn(move |_| {
        let _res0 = resolver.clone();
        let client = client.clone();
        async move {
            let _res1 = _res0.clone();
            Ok::<_, Infallible>(service_fn(move |mut req| {
                let client = client.clone();
                let _res2 = _res1.clone();
                async move {
                    let host_header = req.headers()[http::header::HOST].to_str().unwrap();
                    let authority: http::uri::Authority = host_header.parse().unwrap();
                    let (ip, port) = match _res2.resolve(authority.host()) {
                        Some(r) => r,
                        None => {
                            return Ok::<_, _>(
                                Response::builder()
                                    .status(http::StatusCode::NOT_FOUND)
                                    .body(Body::empty())
                                    .unwrap(),
                            );
                        }
                    };

                    let mut parts = http::uri::Parts::from(req.uri().clone());
                    parts.authority = Some(format!("{}:{}", ip.to_string(), port).parse().unwrap());
                    if parts.scheme.is_none() {
                        parts.scheme = Some("http".parse().unwrap());
                    }

                    *req.uri_mut() = http::uri::Uri::from_parts(parts).unwrap();

                    return client.request(req).await;
                }
            }))
        }
    });
    let server = Server::bind(&addr).serve(make_service);
    if let Err(e) = server.await {
        eprintln!("server error: {:?}", e);
    }
}
