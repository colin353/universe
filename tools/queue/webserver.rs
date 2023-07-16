#[macro_use]
extern crate tmpl;

use auth_client::AuthServer;
use bus::Deserialize;
use largetable_client::LargeTableClient;
use queue_bus::*;
use ws::{Body, Request, Response, Server};

mod render;

static QUEUE: &str = include_str!("html/queue.html");
static TEMPLATE: &str = include_str!("html/template.html");
static INDEX: &str = include_str!("html/index.html");
static DETAIL: &str = include_str!("html/detail.html");

#[derive(Clone)]
pub struct QueueWebServer {
    database: LargeTableClient,
    auth: auth_client::AuthAsyncClient,
    base_url: String,
}

impl QueueWebServer {
    pub fn new(
        database: LargeTableClient,
        auth: auth_client::AuthAsyncClient,
        base_url: String,
    ) -> Self {
        Self {
            database,
            auth,
            base_url,
        }
    }

    fn wrap_template(&self, content: String, subtitle: &str) -> String {
        tmpl::apply(
            TEMPLATE,
            &content!(
                "title" => "queue",
                "subtitle" => subtitle,
                "content" => content
            ),
        )
    }

    async fn queue(&self, queue: &str) -> Response {
        let limit = match self
            .database
            .read::<QueueWindowLimit>(&server_lib::get_queue_window_rowname(), queue, 0)
            .await
        {
            Some(Ok(l)) if l.limit > 20 => (l.limit - 20),
            _ => 0,
        };

        let messages: Vec<_> = self
            .database
            .read_range(
                largetable_client::Filter {
                    row: &server_lib::get_queue_rowname(queue),
                    spec: "",
                    min: &server_lib::get_colname(limit),
                    max: "",
                },
                0,
                25,
            )
            .await
            .unwrap()
            .records
            .into_iter()
            .map(|r| Message::decode(&r.data).unwrap())
            .collect();

        let content = tmpl::apply(
            QUEUE,
            &content!(
                "queue_name" => queue;
                "progress" => messages.iter().rev().filter(|x| !server_lib::is_complete_status(x.status)).map(|x| render::message(x)).collect(),
                "completed" => messages.iter().rev().filter(|x| server_lib::is_complete_status(x.status)).map(|x| render::message(x)).collect()
            ),
        );

        Response::new(Body::from(self.wrap_template(content, queue)))
    }

    async fn message(&self, queue: &str, id: &str) -> Response {
        let id = match id.parse::<u64>() {
            Ok(id) => id,
            Err(_) => return self.not_found(),
        };

        let msg: Message = match self
            .database
            .read(
                &server_lib::get_queue_rowname(queue),
                &server_lib::get_colname(id),
                0,
            )
            .await
        {
            Some(Ok(t)) => t,
            _ => return self.not_found(),
        };

        // Annotate the parent message
        let blocks = if msg.blocks.id > 0 {
            match self
                .database
                .read(
                    &server_lib::get_queue_rowname(&msg.blocks.queue),
                    &server_lib::get_colname(msg.blocks.id),
                    0,
                )
                .await
            {
                Some(Ok(x)) => render::message(&x),
                _ => content!(),
            }
        } else {
            content!()
        };

        let mut subtasks = Vec::new();
        for b in &msg.blocked_by {
            match self
                .database
                .read(
                    &server_lib::get_queue_rowname(&b.queue),
                    &server_lib::get_colname(b.id),
                    0,
                )
                .await
            {
                Some(Ok(x)) => subtasks.push(render::message(&x)),
                _ => continue,
            }
        }

        let content = tmpl::apply(
            DETAIL,
            &content!(
                "message" => render::message(&msg),
                "has_parent" => msg.blocks.id > 0,
                "blocks" => blocks;
                "subtasks" => subtasks
            ),
        );
        Response::new(Body::from(self.wrap_template(content, queue)))
    }

    async fn index(&self, _path: String, _req: Request) -> Response {
        let queues: Vec<_> = self
            .database
            .read_range(
                largetable_client::Filter {
                    row: server_lib::QUEUES,
                    spec: "",
                    min: "",
                    max: "",
                },
                0,
                100,
            )
            .await
            .unwrap()
            .records
            .into_iter()
            .map(|r| r.key)
            .collect();

        let content = tmpl::apply(
            INDEX,
            &content!(
                ;
                "queues" => queues.iter().map(|q| content!("name" => q)).collect::<Vec<_>>()
            ),
        );

        Response::new(Body::from(self.wrap_template(content, "")))
    }

    fn not_found(&self) -> Response {
        Response::new(Body::from(format!("404 not found")))
    }
}

impl Server for QueueWebServer {
    fn respond_future(&self, path: String, req: Request, token: &str) -> ws::ResponseFuture {
        let _self = self.clone();
        let token = token.to_owned();

        Box::pin(async move {
            let result = _self.auth.authenticate(token).await.unwrap();
            if !result.success {
                let challenge = _self
                    .auth
                    .login_then_redirect(format!("{}{}", _self.base_url, path))
                    .await;
                let mut response = Response::new(Body::from("redirect to login"));
                _self.redirect(&challenge.url, &mut response);
                return response;
            }

            let components: Vec<_> = path.split("/").collect();
            if components.len() == 3 && components[1] == "queue" {
                return _self.queue(components[2]).await;
            } else if components.len() == 4 && components[1] == "queue" {
                return _self.message(components[2], components[3]).await;
            }

            return _self.index(path, req).await;
        })
    }
}
