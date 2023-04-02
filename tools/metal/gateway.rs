use hyper::http;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Client, Response, Server};
use std::convert::Infallible;
use std::net::SocketAddr;

type HttpClient = Client<hyper::client::HttpConnector>;

fn extract_token<T>(req: &hyper::Request<T>) -> Result<String, ()> {
    if let Some(h) = req.headers().get(http::header::AUTHORIZATION) {
        if let Ok(token) = h.to_str() {
            return Ok(token.to_string());
        }
    }

    if let Some(h) = req.headers().get(http::header::COOKIE) {
        let cookie = match std::str::from_utf8(h.as_bytes()) {
            Ok(c) => c,
            Err(_) => return Err(()),
        };

        for kv in cookie.split(";") {
            let components: Vec<_> = kv.split("=").collect();
            if components[0].trim() == "token" && components.len() == 2 {
                return Ok(components[1].trim().to_owned());
            }
        }
    }

    return Err(());
}

pub async fn gateway(bind_port: u16, target_port: u16, auth: auth_client::AuthAsyncClient) {
    let addr = SocketAddr::from(([0, 0, 0, 0], bind_port));
    let client = HttpClient::new();
    // Due to a bug in hyper, we must construct two clients, one for H1 and one for H2.
    let h2_client = hyper::Client::builder().http2_only(true).build_http();
    let make_service = make_service_fn(move |_| {
        let auth = auth.clone();
        let client = client.clone();
        let h2_client = h2_client.clone();
        async move {
            let auth = auth.clone();
            Ok::<_, Infallible>(service_fn(move |mut req| {
                let client = client.clone();
                let h2_client = h2_client.clone();
                let auth = auth.clone();
                async move {
                    let token = match extract_token(&req) {
                        Ok(t) => t,
                        Err(_) => {
                            return Ok::<_, _>(
                                Response::builder()
                                    .status(http::StatusCode::UNAUTHORIZED)
                                    .body(Body::from("must provide authorization token"))
                                    .unwrap(),
                            );
                        }
                    };

                    let authorized = match auth.authenticate(token).await {
                        Ok(r) => r.success,
                        Err(e) => {
                            eprintln!("failed to authenticate request with auth service: {e:?}");
                            false
                        }
                    };

                    if !authorized {
                        return Ok::<_, _>(
                            Response::builder()
                                .status(http::StatusCode::UNAUTHORIZED)
                                .body(Body::from("access denied"))
                                .unwrap(),
                        );
                    }

                    let mut parts = http::uri::Parts::from(req.uri().clone());
                    parts.authority = Some(format!("127.0.0.1:{}", target_port).parse().unwrap());
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

#[tokio::main]
async fn main() {
    let auth = auth_client::AuthAsyncClient::new_metal("auth.bus");
    let bus_port = flags::define_flag!(
        "bus_port",
        20202_u16,
        "the port to bind to serve bus traffic"
    );
    flags::parse_flags!(bus_port);

    gateway(bus_port.value(), 20202, auth.clone()).await;
}
