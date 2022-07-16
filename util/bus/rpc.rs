use std::convert::Infallible;

use hyper::http;
use hyper::service::{make_service_fn, service_fn};
use hyper::upgrade::Upgraded;
use hyper::{Body, Client, Method, Request, Response, Server};

type HttpClient = Client<hyper::client::HttpConnector>;

#[derive(Debug)]
enum BusRpcError {
    FailedToBindPort,
}

pub async fn serve<H: BusServer + 'static>(port: u16, handler: H) -> BusRpcError {
    let client = HttpClient::new();
    let make_service = make_service_fn(move |_| {
        let client = client.clone();
        let handler = handler.clone();
        async move {
            let handler = handler.clone();
            Ok::<_, Infallible>(service_fn(move |req| {
                let handler = handler.clone();
                async move {
                    println!("got request = {:?}", req);

                    Ok::<_, Infallible>(
                        Response::builder()
                            .status(http::StatusCode::NOT_FOUND)
                            .body(Body::empty())
                            .unwrap(),
                    )
                }
            }))
        }
    });

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    let server = Server::bind(&addr).serve(make_service);
    if let Err(e) = server.await {
        eprintln!("server error: {:?}", e);
    }
    BusRpcError::FailedToBindPort
}
