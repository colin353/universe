extern crate hyper;

use hyper::rt::Future;
pub use hyper::Body;

pub type Request = hyper::Request<Body>;
pub type Response = hyper::Response<Body>;

pub trait Server: Sync + Send + Clone + 'static {
    fn respond(&self, path: String, Request) -> Response;

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
