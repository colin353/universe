use hyper::http;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Client, Response, Server};
use std::convert::Infallible;
use std::net::SocketAddr;

type HttpClient = Client<hyper::client::HttpConnector>;

pub trait Resolver: Send + Sync + 'static {
    fn resolve(&self, host: &str) -> Option<(std::net::IpAddr, u16)>;
}

fn extract_host<T>(req: &hyper::Request<T>) -> String {
    if let Some(host_header) = req.headers().get(http::header::HOST) {
        if let Ok(auth) = host_header
            .to_str()
            .unwrap()
            .parse::<http::uri::Authority>()
        {
            return auth.host().to_string();
        }
    }
    req.uri().authority().unwrap().host().to_string()
}

// Redirect HTTP traffic to the TLS version (and serve the .well-known directory)
pub async fn handle_http(port: u16, root_dir: Option<std::path::PathBuf>) {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let make_service = make_service_fn(move |_| {
        let root_dir = root_dir.clone();
        async move {
            let root_dir = root_dir.clone();
            Ok::<_, Infallible>(service_fn(move |mut req| {
                let root_dir = root_dir.clone();
                async move {
                    let mut parts = http::uri::Parts::from(req.uri().clone());

                    // Serve .well-known/ directory for SSL cert renewal
                    if let Some(root_dir) = root_dir.as_ref() {
                        if let Some(pq) = parts.path_and_query.as_ref() {
                            if pq.path().starts_with("/.well-known/") {
                                let mut path = root_dir.parent().unwrap().join(&pq.path()[1..]);

                                let path = match path.canonicalize() {
                                    Ok(p) => p,
                                    Err(_) => {
                                        return Ok::<_, _>(
                                            Response::builder()
                                                .status(http::StatusCode::NOT_FOUND)
                                                .body(Body::empty())
                                                .unwrap(),
                                        );
                                    }
                                };

                                // Disallow traversal outside the root dir
                                if !path.starts_with(root_dir) {
                                    return Ok::<_, _>(
                                        Response::builder()
                                            .status(http::StatusCode::NOT_FOUND)
                                            .body(Body::empty())
                                            .unwrap(),
                                    );
                                }

                                return match std::fs::read(path) {
                                    Ok(content) => Ok::<_, _>(
                                        Response::builder()
                                            .status(http::StatusCode::OK)
                                            .body(Body::from(content))
                                            .unwrap(),
                                    ),
                                    Err(_) => Ok::<_, _>(
                                        Response::builder()
                                            .status(http::StatusCode::NOT_FOUND)
                                            .body(Body::empty())
                                            .unwrap(),
                                    ),
                                };
                            }
                        }
                    }

                    // Redirect to HTTPS
                    parts.scheme = Some("https".parse().unwrap());

                    if parts.authority == None {
                        let host = extract_host(&req);
                        parts.authority = Some(format!("{}", host).parse().unwrap());
                    }

                    let mut uri = hyper::Uri::from_parts(parts).unwrap().to_string();
                    let mut resp = Response::builder()
                        .status(http::StatusCode::TEMPORARY_REDIRECT)
                        .body(Body::empty())
                        .unwrap();
                    resp.headers_mut().insert(
                        hyper::header::LOCATION,
                        hyper::header::HeaderValue::from_bytes(uri.as_bytes()).unwrap(),
                    );

                    return Ok::<_, Infallible>(resp);
                }
            }))
        }
    });
    let server = Server::bind(&addr).serve(make_service);
    if let Err(e) = server.await {
        eprintln!("server error: {:?}", e);
    }
}

pub async fn proxy(port: u16, resolver: std::sync::Arc<dyn Resolver>) {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let client = HttpClient::new();
    // Due to a bug in hyper, we must construct two clients, one for H1 and one for H2.
    let h2_client = hyper::Client::builder().http2_only(true).build_http();
    let make_service = make_service_fn(move |_| {
        let _res0 = resolver.clone();
        let client = client.clone();
        let h2_client = h2_client.clone();
        async move {
            let _res1 = _res0.clone();
            Ok::<_, Infallible>(service_fn(move |mut req| {
                let client = client.clone();
                let h2_client = h2_client.clone();
                let _res2 = _res1.clone();
                async move {
                    let host = extract_host(&req);

                    let (ip, port) = match _res2.resolve(&host) {
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
                    parts.scheme = Some("http".parse().unwrap());
                    *req.uri_mut() = http::uri::Uri::from_parts(parts).unwrap();

                    if req.version() == hyper::Version::HTTP_2 {
                        h2_client.request(req).await
                    } else {
                        client.request(req).await
                    }
                }
            }))
        }
    });
    let server = Server::bind(&addr).serve(make_service);
    if let Err(e) = server.await {
        eprintln!("server error: {:?}", e);
    }
}

pub async fn tls_proxy(
    port: u16,
    identity: native_tls::Identity,
    resolver: std::sync::Arc<dyn Resolver>,
) {
    let client = HttpClient::new();
    // Due to a bug in hyper, we must construct two clients, one for H1 and one for H2.
    let h2_client = hyper::Client::builder().http2_only(true).build_http();
    let make_service = || {
        let client = client.clone();
        let h2_client = h2_client.clone();
        let resolver = resolver.clone();
        service_fn(move |mut req| {
            let client = client.clone();
            let h2_client = h2_client.clone();
            let resolver = resolver.clone();
            async move {
                let host = match req.headers().get(http::header::HOST) {
                    // If the host header is set, use that
                    Some(host_header) => host_header.to_str().unwrap(),
                    // If not, use the req.uri
                    None => req.uri().authority().unwrap().host(),
                };

                let (ip, port) = match resolver.resolve(host) {
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
                parts.scheme = Some("http".parse().unwrap());

                *req.uri_mut() = http::uri::Uri::from_parts(parts).unwrap();

                if req.version() == hyper::Version::HTTP_2 {
                    h2_client.request(req).await
                } else {
                    client.request(req).await
                }
            }
        })
    };
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    let mut tcp = tokio::net::TcpListener::bind(&addr).await.unwrap();
    let acceptor = tokio_tls::TlsAcceptor::from(native_tls::TlsAcceptor::new(identity).unwrap());

    loop {
        let (socket, _) = tcp.accept().await.unwrap();
        let acceptor = acceptor.clone();
        let http = hyper::server::conn::Http::new();
        let service = make_service();
        tokio::spawn(async move {
            let tls_stream = acceptor.accept(socket).await.expect("accept error");
            http.serve_connection(tls_stream, service).await
        });
    }
}
