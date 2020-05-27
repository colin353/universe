#[macro_use]
extern crate tmpl;

use auth_client::AuthServer;
use largetable_client::LargeTableClient;
use queue_grpc_rust::*;
use ws::{Body, Request, Response, Server};

mod render;

static QUEUE: &str = include_str!("html/queue.html");
static TEMPLATE: &str = include_str!("html/template.html");
static INDEX: &str = include_str!("html/index.html");
static DETAIL: &str = include_str!("html/detail.html");

#[derive(Clone)]
pub struct QueueWebServer<C: LargeTableClient + Send + Sync + Clone + 'static> {
    database: C,
    auth: auth_client::AuthClient,
    base_url: String,
}

impl<C: LargeTableClient + Clone + Send + Sync + 'static> QueueWebServer<C> {
    pub fn new(database: C, auth: auth_client::AuthClient, base_url: String) -> Self {
        Self {
            database,
            auth,
            base_url,
        }
    }

    fn wrap_template(&self, content: String) -> String {
        tmpl::apply(
            TEMPLATE,
            &content!(
                "title" => "tasks",
                "content" => content
            ),
        )
    }

    fn queue(&self, queue: &str) -> Response {
        let mut messages: Vec<_> = largetable_client::LargeTableScopedIterator::<Message, C>::new(
            &self.database,
            server_lib::get_queue_rowname(queue),
            String::new(),
            String::new(),
            String::new(),
            0,
        )
        .map(|(_, m)| m)
        .take(25)
        .collect();

        let content = tmpl::apply(
            QUEUE,
            &content!(
                ;
                "progress" => messages.iter().filter(|x| !server_lib::is_complete_status(x.get_status())).map(|x| render::message(x)).collect(),
                "completed" => messages.iter().filter(|x| server_lib::is_complete_status(x.get_status())).map(|x| render::message(x)).collect()
            ),
        );

        Response::new(Body::from(self.wrap_template(content)))
    }

    fn index(&self, _path: String, _req: Request) -> Response {
        let queues: Vec<_> = largetable_client::LargeTableScopedIterator::<Message, C>::new(
            &self.database,
            server_lib::QUEUES.to_string(),
            String::new(),
            String::new(),
            String::new(),
            0,
        )
        .map(|(k, _)| k)
        .collect();

        let content = tmpl::apply(
            INDEX,
            &content!(
                ;
                "queues" => queues.iter().map(|q| content!("name" => q)).collect::<Vec<_>>()
            ),
        );

        Response::new(Body::from(self.wrap_template(content)))
    }
}

impl<C: LargeTableClient + Send + Sync + Clone + 'static> Server for QueueWebServer<C> {
    fn respond(&self, path: String, req: Request, token: &str) -> Response {
        let result = self.auth.authenticate(token.to_owned());
        if !result.get_success() {
            let challenge = self
                .auth
                .login_then_redirect(format!("{}{}", self.base_url, path));
            let mut response = Response::new(Body::from("redirect to login"));
            self.redirect(challenge.get_url(), &mut response);
            return response;
        }

        let components: Vec<_> = path.split("/").collect();
        if components.len() == 3 && components[1] == "queue" {
            return self.queue(components[2]);
        }

        return self.index(path, req);
    }
}
