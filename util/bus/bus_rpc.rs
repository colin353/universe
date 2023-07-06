use futures::TryFutureExt;
use hyper::body::{Buf, HttpBody};
use hyper::client::{connect::Connect, HttpConnector};
use hyper::http;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Client, Request, Response, Server};

use bus::Stream;

use std::collections::VecDeque;
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::Mutex;

mod metal;
pub use metal::MetalAsyncClient;

pub async fn serve<H: bus::BusAsyncServer + 'static>(port: u16, handler: H) -> bus::BusRpcError {
    let make_service = make_service_fn(move |_| {
        let handler = handler.clone();
        async move {
            let handler = handler.clone();
            Ok::<_, Infallible>(service_fn(move |req: Request<Body>| {
                let handler = handler.clone();
                async move {
                    let is_stream = req.headers().contains_key("bus-stream");

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

                    if is_stream {
                        let (sink, rec) = bus::BusSink::new();
                        tokio::task::spawn(async move {
                            handler
                                .serve_stream(&service, &method, &payload, sink)
                                .await
                        });

                        return Ok::<_, Infallible>(
                            Response::builder()
                                .status(http::StatusCode::OK)
                                .body(hyper::Body::wrap_stream(rec))
                                .unwrap(),
                        );
                    }

                    handler
                        .serve(&service, &method, &payload)
                        .and_then(|data| {
                            std::future::ready(Ok::<_, bus::BusRpcError>(
                                Response::builder()
                                    .status(http::StatusCode::OK)
                                    .body(data.into())
                                    .unwrap(),
                            ))
                        })
                        .or_else(|e| {
                            std::future::ready(Ok::<_, Infallible>(match e {
                                bus::BusRpcError::NotImplemented => Response::builder()
                                    .status(http::StatusCode::NOT_IMPLEMENTED)
                                    .body(Body::empty())
                                    .unwrap(),
                                bus::BusRpcError::ServiceNameDidNotMatch => Response::builder()
                                    .status(http::StatusCode::NOT_FOUND)
                                    .body(Body::empty())
                                    .unwrap(),
                                _ => Response::builder()
                                    .status(http::StatusCode::INTERNAL_SERVER_ERROR)
                                    .body(Body::empty())
                                    .unwrap(),
                            }))
                        })
                        .await
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
    headers: Vec<(hyper::header::HeaderName, String)>,
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
                headers: Vec::new(),
            }),
        }
    }
}

impl<T> HyperClient<T> {
    pub fn add_header(&mut self, header: hyper::header::HeaderName, value: String) {
        Arc::get_mut(&mut self.inner)
            .unwrap()
            .headers
            .push((header, value));
    }
}

impl<T> HyperSyncClient<T> {
    pub fn add_header(&mut self, header: hyper::header::HeaderName, value: String) {
        self.inner.add_header(header, value);
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
                headers: Vec::new(),
            }),
        }
    }
}

#[derive(Clone)]
struct BusStream {
    inner: Arc<Mutex<BusStreamInner>>,
}

struct BusStreamInner {
    bytes: Response<Body>,
    size: Option<usize>,
    buffer: VecDeque<u8>,
}

impl BusStream {
    fn new(r: Response<Body>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(BusStreamInner {
                bytes: r,
                size: None,
                buffer: VecDeque::new(),
            })),
        }
    }

    async fn next(&mut self) -> Option<Vec<u8>> {
        loop {
            let mut _self = self.inner.lock().await;

            if let Some(len) = _self.size {
                if _self.buffer.len() > len {
                    let mut out = _self.buffer.split_off(len);
                    std::mem::swap(&mut out, &mut _self.buffer);
                    _self.size = None;
                    return Some(Vec::from(out));
                }
            } else if _self.buffer.len() > 4 {
                _self.size = Some(u32::from_le_bytes([
                    _self.buffer.pop_front().unwrap(),
                    _self.buffer.pop_front().unwrap(),
                    _self.buffer.pop_front().unwrap(),
                    _self.buffer.pop_front().unwrap(),
                ]) as usize);
            }

            let data = match _self.bytes.data().await {
                Some(Ok(d)) => d,
                None | Some(Err(_)) => return None,
            };

            for byte in data.iter() {
                _self.buffer.push_back(*byte);
            }
        }
    }
}

impl Stream for BusStream {
    fn next(&mut self) -> std::pin::Pin<Box<dyn std::future::Future<Output = Option<Vec<u8>>>>> {
        let mut _self = self.clone();
        Box::pin(async move { _self.next().await })
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

        let mut req = hyper::Request::builder()
            .method("POST")
            .uri(uri)
            .body(hyper::Body::from(data))
            .map_err(|e| bus::BusRpcError::InternalError(format!("{:?}", e)))?;

        for (header, value) in &self.inner.headers {
            req.headers_mut().insert(
                header,
                hyper::header::HeaderValue::from_bytes(value.as_bytes()).unwrap(),
            );
        }

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

    async fn stream_async(&self, uri: &str, data: Vec<u8>) -> Result<BusStream, bus::BusRpcError> {
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

        let mut req = hyper::Request::builder()
            .method("POST")
            .uri(uri)
            .body(hyper::Body::from(data))
            .map_err(|e| bus::BusRpcError::InternalError(format!("{:?}", e)))?;

        req.headers_mut().insert(
            hyper::header::HeaderName::from_static("bus-stream"),
            hyper::header::HeaderValue::from_bytes("1".as_bytes()).unwrap(),
        );

        for (header, value) in &self.inner.headers {
            req.headers_mut().insert(
                header,
                hyper::header::HeaderValue::from_bytes(value.as_bytes()).unwrap(),
            );
        }

        let resp = self
            .inner
            .client
            .request(req)
            .await
            .map_err(|e| bus::BusRpcError::ConnectionError(format!("{:?}", e)))?;

        Ok(BusStream::new(resp))
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

    fn request_stream(
        &self,
        uri: &'static str,
        data: Vec<u8>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                Output = Result<std::pin::Pin<Box<dyn Stream>>, bus::BusRpcError>,
            >,
        >,
    > {
        let _self = self.clone();
        Box::pin(async move {
            _self.stream_async(uri, data).await.map(|r| {
                let result: std::pin::Pin<Box<dyn Stream>> = Box::pin(r);
                result
            })
        })
    }
}
