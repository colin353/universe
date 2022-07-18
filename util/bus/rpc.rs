use hyper::body::Buf;
use hyper::http;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Client, Request, Response, Server};

use std::convert::Infallible;

type HttpClient = Client<hyper::client::HttpConnector>;

pub async fn serve<H: bus::BusServer + 'static>(port: u16, handler: H) -> bus::BusRpcError {
    let make_service = make_service_fn(move |_| {
        let handler = handler.clone();
        async move {
            let handler = handler.clone();
            Ok::<_, Infallible>(service_fn(move |req: Request<Body>| {
                let handler = handler.clone();
                async move {
                    let mut iter = req.uri().path().split("/");
                    let (service, method) =
                        match (iter.next(), iter.next(), iter.next(), iter.next()) {
                            (Some(""), Some(service), Some(method), None) => {
                                (service.to_string(), method.to_string())
                            }
                            _ => {
                                return Ok(Response::builder()
                                    .status(http::StatusCode::NOT_FOUND)
                                    .body(Body::empty())
                                    .unwrap())
                            }
                        };

                    let payload: Vec<u8> = match hyper::body::to_bytes(req).await {
                        Ok(p) => p.to_vec(),
                        Err(_) => {
                            return Ok(Response::builder()
                                .status(http::StatusCode::BAD_REQUEST)
                                .body(Body::empty())
                                .unwrap())
                        }
                    };

                    Ok::<_, Infallible>(match handler.serve(&service, &method, &payload) {
                        Ok(data) => Response::builder()
                            .status(http::StatusCode::OK)
                            .body(data.into())
                            .unwrap(),
                        Err(bus::BusRpcError::NotImplemented) => Response::builder()
                            .status(http::StatusCode::NOT_IMPLEMENTED)
                            .body(Body::empty())
                            .unwrap(),
                        Err(bus::BusRpcError::ServiceNameDidNotMatch) => Response::builder()
                            .status(http::StatusCode::NOT_FOUND)
                            .body(Body::empty())
                            .unwrap(),
                        _ => Response::builder()
                            .status(http::StatusCode::INTERNAL_SERVER_ERROR)
                            .body(Body::empty())
                            .unwrap(),
                    })
                }
            }))
        }
    });

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    let server = Server::bind(&addr).serve(make_service);
    if let Err(e) = server.await {
        eprintln!("server error: {:?}", e);
    }
    bus::BusRpcError::FailedToBindPort
}

pub struct HyperClient {
    host: String,
    port: u16,
    client: HttpClient,
    executor: tokio::runtime::Runtime,
}

impl HyperClient {
    pub fn new(host: String, port: u16) -> Self {
        let executor = tokio::runtime::Builder::new()
            .threaded_scheduler()
            .enable_all()
            .build()
            .unwrap();

        HyperClient {
            host,
            port,
            executor,
            client: hyper::Client::builder().http2_only(true).build_http(),
        }
    }

    async fn request_async(&self, uri: &str, data: Vec<u8>) -> Result<Vec<u8>, bus::BusRpcError> {
        let uri = match hyper::Uri::builder()
            .scheme("http")
            .authority(format!("{}:{}", self.host, self.port))
            .path_and_query(uri)
            .build()
        {
            Ok(u) => u,
            Err(e) => return Err(bus::BusRpcError::InternalError(format!("{:?}", e))),
        };

        let req = hyper::Request::builder()
            .method("POST")
            .uri(uri)
            .body(hyper::Body::from(data))
            .map_err(|e| bus::BusRpcError::InternalError(format!("{:?}", e)))?;

        let resp = self
            .client
            .request(req)
            .await
            .map_err(|e| bus::BusRpcError::ConnectionError(format!("{:?}", e)))?;

        let mut out = hyper::body::aggregate(resp)
            .await
            .map_err(|e| bus::BusRpcError::ConnectionError(format!("{:?}", e)))?;

        Ok(out.to_bytes().to_vec())
    }
}

impl bus::BusClient for HyperClient {
    fn request(&self, uri: &str, data: Vec<u8>) -> Result<Vec<u8>, bus::BusRpcError> {
        self.executor.enter(|| {
            let handle = self.executor.handle();
            handle.block_on(async { self.request_async(uri, data).await })
        })
    }
}
