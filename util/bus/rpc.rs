use hyper::body::Buf;
use hyper::client::{connect::Connect, HttpConnector};
use hyper::http;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Client, Request, Response, Server};

use std::convert::Infallible;
use std::sync::Arc;

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

pub struct HyperSyncClient<T> {
    inner: HyperClient<T>,
    executor: tokio::runtime::Runtime,
}

pub struct HyperClientInner<T> {
    host: String,
    port: u16,
    client: Client<T>,
    use_tls: bool,
}

#[derive(Clone)]
pub struct HyperClient<T> {
    inner: Arc<HyperClientInner<T>>,
}

impl HyperSyncClient<HttpConnector> {
    pub fn new(host: String, port: u16) -> Self {
        let executor = tokio::runtime::Builder::new()
            .threaded_scheduler()
            .enable_all()
            .build()
            .unwrap();

        HyperSyncClient {
            inner: HyperClient::new(host, port),
            executor,
        }
    }
}

impl HyperSyncClient<hyper_tls::HttpsConnector<HttpConnector>> {
    pub fn new_tls(host: String, port: u16) -> Self {
        let executor = tokio::runtime::Builder::new()
            .threaded_scheduler()
            .enable_all()
            .build()
            .unwrap();

        HyperSyncClient {
            inner: HyperClient::new_tls(host, port),
            executor,
        }
    }
}

impl HyperClient<HttpConnector> {
    pub fn new(host: String, port: u16) -> Self {
        HyperClient {
            inner: Arc::new(HyperClientInner {
                host,
                port,
                client: hyper::Client::builder().http2_only(true).build_http(),
                use_tls: false,
            }),
        }
    }
}

impl HyperClient<hyper_tls::HttpsConnector<HttpConnector>> {
    pub fn new_tls(host: String, port: u16) -> Self {
        let https = hyper_tls::HttpsConnector::new();
        HyperClient {
            inner: Arc::new(HyperClientInner {
                host,
                port,
                client: hyper::Client::builder().http2_only(true).build(https),
                use_tls: true,
            }),
        }
    }
}

impl<T: Connect + Clone + Send + Sync + 'static> HyperClient<T> {
    async fn request_async(&self, uri: &str, data: Vec<u8>) -> Result<Vec<u8>, bus::BusRpcError> {
        let builder = hyper::Uri::builder();
        let builder = if self.inner.use_tls {
            builder.scheme("https")
        } else {
            builder.scheme("http")
        };
        let uri = match builder
            .authority(format!("{}:{}", self.inner.host, self.inner.port))
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
            .inner
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

impl<T: Connect + Clone + Send + Sync + 'static> bus::BusClient for HyperSyncClient<T> {
    fn request(&self, uri: &str, data: Vec<u8>) -> Result<Vec<u8>, bus::BusRpcError> {
        self.executor.enter(|| {
            let handle = self.executor.handle();
            handle.block_on(async { self.inner.request_async(uri, data).await })
        })
    }
}

impl<T: Connect + Clone + Send + Sync + 'static> bus::BusAsyncClient for HyperClient<T> {
    fn request(
        &self,
        uri: &'static str,
        data: Vec<u8>,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Vec<u8>, bus::BusRpcError>> + Send>,
    > {
        let _self = self.clone();
        Box::pin(async move { _self.request_async(uri, data).await })
    }
}
