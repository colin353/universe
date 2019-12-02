extern crate futures;
extern crate hyper;
extern crate rand;
use rand::Rng;

use futures::future;
use hyper::header::HeaderValue;
use hyper::header::{COOKIE, LOCATION, SET_COOKIE};
use hyper::http::StatusCode;
use hyper::rt::Future;
use hyper::service::service_fn;
pub use hyper::Body;

pub type Request = hyper::Request<Body>;
pub type Response = hyper::Response<Body>;
pub type ResponseFuture = Box<dyn Future<Item = Response, Error = std::io::Error> + Send>;

use std::io::Read;

fn extract_key(h: &HeaderValue, key: &str) -> Option<String> {
    let cookie = match std::str::from_utf8(h.as_bytes()) {
        Ok(c) => c,
        Err(_) => return None,
    };
    for kv in cookie.split(";") {
        let components: Vec<_> = kv.split("=").collect();
        if components[0].trim() == key && components.len() == 2 {
            return Some(components[1].trim().to_owned());
        }
    }
    None
}

fn random_string() -> String {
    let mut rng = rand::thread_rng();
    format!(
        "{}{}{}{}",
        rng.gen::<u64>(),
        rng.gen::<u64>(),
        rng.gen::<u64>(),
        rng.gen::<u64>()
    )
}

pub trait Server: Sync + Send + Clone + 'static {
    fn respond(&self, path: String, req: Request, session_key: &str) -> Response {
        panic!("not implemented")
    }

    fn respond_future(&self, path: String, req: Request, session_key: &str) -> ResponseFuture {
        Box::new(future::ok(self.respond(path, req, session_key)))
    }

    fn serve_static_files(&self, path: String, prefix: &str, static_directory: &str) -> Response {
        if !path.starts_with(prefix) || path.contains("..") {
            return self.not_found(path);
        }
        let final_path = format!("{}{}", static_directory, &path[prefix.len() - 1..]);
        let mut file = match std::fs::File::open(final_path) {
            Ok(f) => f,
            Err(_) => return self.not_found(path),
        };
        let mut contents = String::new();
        if let Err(_) = file.read_to_string(&mut contents) {
            return self.not_found(path);
        }
        return Response::new(Body::from(contents));
    }

    fn not_found(&self, path: String) -> Response {
        Response::new(Body::from(format!("404 not found: path {}", path)))
    }

    fn set_cookie(&self, new_token: &str, response: &mut Response) {
        response.headers_mut().insert(
            SET_COOKIE,
            HeaderValue::from_bytes(format!("token={}", new_token).as_bytes()).unwrap(),
        );
    }

    fn redirect(&self, location: &str, response: &mut Response) {
        *response.status_mut() = StatusCode::TEMPORARY_REDIRECT;
        response.headers_mut().insert(
            LOCATION,
            HeaderValue::from_bytes(location.as_bytes()).unwrap(),
        );
    }

    fn serve(self, port: u16) {
        let addr = ([0, 0, 0, 0], port).into();
        let self_clone = self.clone();
        let server = hyper::Server::bind(&addr);

        hyper::rt::run(
            server
                .serve(move || {
                    let s = self_clone.clone();
                    service_fn(move |req| {
                        let mut maybe_session_key = None;
                        if let Some(c) = req.headers().get(COOKIE) {
                            maybe_session_key = extract_key(c, "token");
                        }
                        let (has_cookie, session_key) = match maybe_session_key {
                            Some(k) => (true, k),
                            None => (false, random_string()),
                        };

                        let mut response =
                            s.respond_future(req.uri().path().into(), req, &session_key);

                        /*if !has_cookie {
                            s.set_cookie(&session_key, &mut response);
                        }*/

                        response
                    })
                })
                .map_err(|e| println!("error: {}", e)),
        );
    }
}
