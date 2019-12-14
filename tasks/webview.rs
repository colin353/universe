#[macro_use]
extern crate tmpl;
extern crate largetable_client;
extern crate task_db_client;
extern crate ws;

use largetable_client::LargeTableClient;
use ws::{Body, Request, Response, Server};

#[derive(Clone)]
pub struct TaskWebServer<C: LargeTableClient + Send + Sync + Clone + 'static> {
    client: task_db_client::TaskClient<C>,
}

impl<C: LargeTableClient + Clone + Send + Sync + 'static> TaskWebServer<C> {
    pub fn new(database: C) -> Self {
        Self {
            client: task_db_client::TaskClient::new(database),
        }
    }

    fn index(&self, _path: String, _req: Request) -> Response {
        Response::new(Body::from("hello task"))
    }
}

impl<C: LargeTableClient + Send + Sync + Clone + 'static> Server for TaskWebServer<C> {
    fn respond(&self, path: String, req: Request, token: &str) -> Response {
        self.index(path, req)
    }
}
