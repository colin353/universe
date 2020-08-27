extern crate futures;
extern crate hyper;
extern crate rand;
use rand::Rng;

use futures::future;
use hyper::header::HeaderValue;
use hyper::header::{
    ACCESS_CONTROL_ALLOW_ORIGIN, CACHE_CONTROL, CONTENT_TYPE, COOKIE, LOCATION, SET_COOKIE,
};
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

    fn serve_static_files(&self, path: String, prefix: &str, static_directories: &str) -> Response {
        if !path.starts_with(prefix) || path.contains("..") {
            return self.not_found(path);
        }

        let subdir_path = if prefix.ends_with("/") {
            &path[prefix.len() - 1..]
        } else {
            &path[prefix.len()..]
        };

        // If the static_directories field contains several directories, we look in each one in
        // series before returning 404.
        for static_directory in static_directories.split(",") {
            let final_path = format!("{}{}", static_directory, subdir_path);
            let mut file = match std::fs::File::open(final_path) {
                Ok(f) => f,
                Err(_) => continue,
            };
            let mut contents = String::new();
            if let Err(_) = file.read_to_string(&mut contents) {
                continue;
            }
            let mut response = Response::new(Body::from(contents));

            let mut content_type = None;
            if path.ends_with(".js") || path.ends_with(".mjs") {
                content_type = Some("text/javascript");
            } else if path.ends_with(".css") {
                content_type = Some("text/css");
            } else if path.ends_with(".json") {
                content_type = Some("application/json");
            }

            if let Some(c) = content_type {
                response
                    .headers_mut()
                    .insert(CONTENT_TYPE, HeaderValue::from_bytes(c.as_bytes()).unwrap());
            }

            response.headers_mut().insert(
                CACHE_CONTROL,
                HeaderValue::from_bytes("max-age=100000".as_bytes()).unwrap(),
            );
            response.headers_mut().insert(
                ACCESS_CONTROL_ALLOW_ORIGIN,
                HeaderValue::from_bytes("*".as_bytes()).unwrap(),
            );
            return response;
        }

        self.not_found(path)
    }

    fn not_found(&self, path: String) -> Response {
        let mut response = Response::new(Body::from(format!("404 not found: path {}", path)));
        *response.status_mut() = StatusCode::NOT_FOUND;
        response
    }

    fn set_cookie_for_domain(&self, new_token: &str, domain: &str, response: &mut Response) {
        response.headers_mut().insert(
            SET_COOKIE,
            HeaderValue::from_bytes(
                format!("token={}; Domain={}; Path=/; HttpOnly", new_token, domain).as_bytes(),
            )
            .unwrap(),
        );
    }

    fn set_cookie(&self, new_token: &str, response: &mut Response) {
        response.headers_mut().insert(
            SET_COOKIE,
            HeaderValue::from_bytes(format!("token={}; Path=/; HttpOnly", new_token).as_bytes())
                .unwrap(),
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

                        s.respond_future(req.uri().path().into(), req, &session_key)
                    })
                })
                .map_err(|e| println!("error: {}", e)),
        );
    }
}
