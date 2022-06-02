#[macro_use]
extern crate flags;
extern crate ws;
extern crate x20_client;

use ws::{Body, Request, Response, Server};

static INDEX: &str = include_str!("index.html");
static SCRIPT: &str = include_str!("x20.sh");

#[derive(Clone)]
struct X20Webserver {
    client: x20_client::X20Client,
}

impl X20Webserver {
    fn new(client: x20_client::X20Client) -> Self {
        Self { client: client }
    }

    fn index(&self, _path: String, _req: Request) -> Response {
        Response::new(Body::from(INDEX))
    }

    fn script(&self) -> Response {
        Response::new(Body::from(SCRIPT))
    }

    fn binary(&self) -> Response {
        let url = match self
            .client
            .get_binaries()
            .expect("couldn't get binaries!")
            .into_iter()
            .find(|b| b.get_name() == "x20")
        {
            Some(mut x) => x.take_url(),
            None => return Response::new(Body::from("404 not found")),
        };

        let mut response = Response::new(Body::from(""));
        self.redirect(&url, &mut response);
        response
    }
}

impl Server for X20Webserver {
    fn respond(&self, path: String, req: Request, token: &str) -> Response {
        if path == "/x20" {
            return self.binary();
        }

        if path == "/x20.sh" {
            return self.script();
        }

        self.index(path, req)
    }
}

#[tokio::main]
async fn main() {
    let port = define_flag!("port", 50000, "the port to bind to");
    let x20_hostname = define_flag!(
        "x20_hostname",
        String::from("x20"),
        "the hostname of the x20 service"
    );
    let x20_port = define_flag!("x20_port", 8009, "the port of the x20 service");
    parse_flags!(port, x20_hostname, x20_port);

    let client = x20_client::X20Client::new(&x20_hostname.value(), x20_port.value(), String::new());
    ws::serve(X20Webserver::new(client), port.value()).await;
}
