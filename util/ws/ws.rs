extern crate hyper;

use hyper::rt::Future;
pub use hyper::Body;

pub type Request = hyper::Request<Body>;
pub type Response = hyper::Response<Body>;

use std::io::Read;

pub trait Server: Sync + Send + Clone + 'static {
    fn respond(&self, path: String, Request) -> Response;

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

    fn serve(self, port: u16) {
        let addr = ([127, 0, 0, 1], port).into();
        let self_clone = self.clone();
        let server = hyper::Server::bind(&addr)
            .serve(move || {
                let s = self_clone.clone();
                hyper::service::service_fn_ok(move |req: Request| {
                    s.respond(req.uri().path().into(), req)
                })
            })
            .map_err(|_| ());;

        hyper::rt::run(server);
    }
}
