#[macro_use]
extern crate tmpl;
extern crate largetable_client;
extern crate task_client;
extern crate tasks_grpc_rust;
extern crate ws;

mod render;

use largetable_client::LargeTableClient;
use tasks_grpc_rust::{Status, TaskArgument, TaskArtifact, TaskStatus};
use ws::{Body, Request, Response, Server};

static TEMPLATE: &str = include_str!("template.html");
static INDEX: &str = include_str!("index.html");
static DETAIL: &str = include_str!("detail.html");

#[derive(Clone)]
pub struct TaskWebServer<C: LargeTableClient + Send + Sync + Clone + 'static> {
    client: task_client::TaskClient<C>,
}

impl<C: LargeTableClient + Clone + Send + Sync + 'static> TaskWebServer<C> {
    pub fn new(database: C) -> Self {
        Self {
            client: task_client::TaskClient::new(database),
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

    fn index(&self, _path: String, _req: Request) -> Response {
        let mut progress = Vec::new();
        let mut completed = Vec::new();
        for task in self.client.list_tasks() {
            let rendered = render::status(&task);
            if task.get_status() == Status::CREATED || task.get_status() == Status::STARTED {
                progress.push(rendered);
            } else {
                completed.push(rendered);
            }
        }

        let page = tmpl::apply(
            INDEX,
            &content!(;
                "progress" => progress
                "completed" => completed
            ),
        );

        Response::new(Body::from(self.wrap_template(page)))
    }

    fn task_detail(&self, path: String, _req: Request) -> Response {
        let task = match self.client.read(&path[1..]) {
            Some(t) => t,
            None => return self.not_found(),
        };

        let page = tmpl::apply(DETAIL, &render::status(&task));
        Response::new(Body::from(self.wrap_template(page)))
    }

    fn not_found(&self) -> Response {
        Response::new(Body::from("not found"))
    }
}

impl<C: LargeTableClient + Send + Sync + Clone + 'static> Server for TaskWebServer<C> {
    fn respond(&self, path: String, req: Request, token: &str) -> Response {
        if path == "/" {
            return self.index(path, req);
        }

        self.task_detail(path, req)
    }
}
