use std::sync::Arc;

use crate::render;
use ws::{Body, Request, Response, Server};

static TEMPLATE: &str = include_str!("html/template.html");
static INDEX: &str = include_str!("html/index.html");
static DETAIL: &str = include_str!("html/detail.html");
static RESULTS: &str = include_str!("html/results.html");

#[derive(Clone)]
pub struct SearchWebserver<A> {
    static_dir: String,
    auth: A,
    searcher: Arc<search_lib::Searcher>,
    base_url: String,
}

impl<A> SearchWebserver<A>
where
    A: auth_client::AuthServer,
{
    pub fn new(
        searcher: Arc<search_lib::Searcher>,
        static_dir: String,
        base_url: String,
        auth: A,
    ) -> Self {
        Self {
            static_dir: static_dir,
            auth: auth,
            base_url: base_url,
            searcher: searcher,
        }
    }

    fn wrap_template(&self, header: bool, query: &str, content: String) -> String {
        tmpl::apply(
            TEMPLATE,
            &content!(
                "title" => "code search",
                "show_header" => header,
                "query" => query,
                "content" => content),
        )
    }

    fn results(&self, keywords: &str, path: String, req: Request) -> Response {
        let candidates = self.searcher.search(keywords);

        if candidates.len() == 1 {
            // Only one search result! Skip right to the detail page.
            let mut response = Response::new(Body::from(""));
            self.redirect(
                &format!(
                    "/{}?q={}",
                    candidates[0].get_filename(),
                    ws_utils::urlencode(keywords)
                ),
                &mut response,
            );
            return response;
        }

        let page = tmpl::apply(
            RESULTS,
            &content!("query" => keywords; "results" => candidates.iter().map(|r| render::result(r)).collect()),
        );
        Response::new(Body::from(self.wrap_template(true, keywords, page)))
    }

    fn detail(&self, query: &str, path: String, req: Request) -> Response {
        let file = match self.searcher.get_document(&path[1..]) {
            Some(f) => f,
            None => return self.not_found(path, req),
        };
        let page = tmpl::apply(DETAIL, &render::file(&file));
        Response::new(Body::from(self.wrap_template(true, query, page)))
    }

    fn index(&self, path: String, req: Request) -> Response {
        let page = tmpl::apply(INDEX, &content!());
        Response::new(Body::from(self.wrap_template(false, "", page)))
    }

    fn not_found(&self, path: String, _req: Request) -> Response {
        Response::new(Body::from(format!("404 not found: path {}", path)))
    }
}

impl<A> Server for SearchWebserver<A>
where
    A: auth_client::AuthServer,
{
    fn respond(&self, path: String, req: Request, token: &str) -> Response {
        if path.starts_with("/static/") {
            return self.serve_static_files(path, "/static/", &self.static_dir);
        }

        let result = self.auth.authenticate(token.to_owned());
        if !result.get_success() {
            let challenge = self
                .auth
                .login_then_redirect(format!("{}{}", self.base_url, path));
            let mut response = Response::new(Body::from("redirect to login"));
            self.set_cookie(challenge.get_token(), &mut response);
            self.redirect(challenge.get_url(), &mut response);
            return response;
        }

        let mut query = String::new();
        if let Some(q) = req.uri().query() {
            let params = ws_utils::parse_params(q);
            if let Some(keywords) = params.get("q") {
                query = keywords.to_owned();
            }
        };

        if path.len() > 1 {
            return self.detail(&query, path, req);
        }

        if query.len() > 0 {
            return self.results(&query, path, req);
        }

        self.index(path, req)
    }
}
