use std::sync::Arc;

use crate::render;
use ws::{Body, Request, Response, Server};

static TEMPLATE: &str = include_str!("html/template.html");
static SIDEBAR: &str = include_str!("html/sidebar.html");
static INDEX: &str = include_str!("html/index.html");
static DETAIL: &str = include_str!("html/detail.html");
static DETAIL_MD: &str = include_str!("html/markdown.html");
static DETAIL_FOLDER: &str = include_str!("html/detail_folder.html");
static DETAIL_TEMPLATE: &str = include_str!("html/detail_template.html");
static RESULTS: &str = include_str!("html/results.html");
static FAVICON: &[u8] = include_bytes!("html/favicon.png");
static OPENSEARCH: &str = include_str!("html/opensearch.xml");

#[derive(Clone)]
pub struct SearchWebserver {
    static_dir: String,
    searcher: Arc<search_lib::Searcher>,
    base_url: String,
    settings: tmpl::ContentsMap,
}

impl SearchWebserver {
    pub fn new(
        searcher: Arc<search_lib::Searcher>,
        static_dir: String,
        base_url: String,
        js_src: String,
    ) -> Self {
        Self {
            static_dir: static_dir,
            base_url: base_url,
            searcher: searcher,
            settings: content!("js_src" => js_src),
        }
    }

    fn wrap_template(
        &self,
        header: bool,
        query: &str,
        content: String,
        request_time: u32,
        align_left: bool,
    ) -> String {
        tmpl::apply_with_settings(
            TEMPLATE,
            content!(
                "title" => "code search",
                "show_header" => header,
                "align_left" => match align_left {
                    true => " left-aligned",
                    false => "",
                },
                "query" => query,
                "request_time" => request_time,
                "content" => content),
            &self.settings,
        )
    }

    fn results(&self, keywords: &str, _path: String, _req: Request) -> Response {
        let start = std::time::Instant::now();
        let mut results = self.searcher.search(keywords);

        if results.get_candidates().len() == 1 {
            // Only one search result! Skip right to the detail page.
            let mut response = Response::new(Body::from(""));
            self.redirect(
                &format!(
                    "/{}?q={}#L{}",
                    results.get_candidates()[0].get_filename(),
                    ws_utils::urlencode(keywords),
                    results.get_candidates()[0].get_jump_to_line() + 1,
                ),
                &mut response,
            );
            return response;
        }

        let mut content = content!(
            "has_feature_entity" => false,
            "query" => keywords;
            "results" => results.get_candidates().iter().map(|r| render::result(r)).collect(),
            "languages" => results.take_languages().iter().map(|x| content!("name" => x)).collect(),
            "prefixes" => results.take_prefixes().iter().map(|x| content!("name" => x)).collect()
        );

        if results.get_entities().len() > 0 {
            content.insert("has_feature_entity", true);
            content.insert("feature_entity", render::entity(&results.get_entities()[0]));
        }

        let page = tmpl::apply_with_settings(RESULTS, content, &self.settings);
        Response::new(Body::from(self.wrap_template(
            true,
            keywords,
            page,
            start.elapsed().as_millis() as u32,
            true,
        )))
    }

    fn suggest(&self, query: &str, is_opensearch: bool, _req: Request) -> Response {
        let mut response = self.searcher.suggest(query);
        let mut output = Vec::new();

        for entity in response.take_entities().into_iter().take(3) {
            output.push(render::entity_info(&entity));
        }

        for keyword in response.take_suggestions().into_iter() {
            let mut obj = json::object::Object::new();
            obj["name"] = keyword.into();
            output.push(obj.into());

            if output.len() > 10 {
                break;
            }
        }

        if is_opensearch {
            // The opensearch format (for whatever reason) is an array where the first
            // element is a string with the original query and the second element is an
            // array of suggestions.
            let mut container = Vec::new();
            container.push(json::JsonValue::String(query.to_owned()));
            container.push(json::JsonValue::Array(output));
            return Response::new(Body::from(json::stringify(container)));
        }

        Response::new(Body::from(json::stringify(output)))
    }

    fn info(&self, query: &str, _req: Request) -> Response {
        let response = self.searcher.search(query);
        let mut output = Vec::new();
        for candidate in response.get_candidates() {
            if candidate.get_jump_to_line() > 0 {
                output.push(format!(
                    "{}#L{}",
                    candidate.get_filename(),
                    candidate.get_jump_to_line() + 1
                ));
            } else {
                output.push(candidate.get_filename().to_owned());
            }
        }

        Response::new(Body::from(json::stringify(output)))
    }

    fn detail(&self, query: &str, path: String, req: Request) -> Response {
        let (file, content) = match self.searcher.get_document(&path[1..]) {
            Some(f) => f,
            None => return self.not_found(path, req),
        };

        let content = unsafe { std::str::from_utf8_unchecked(content) };

        println!("pagerank: {:?}", file.get_page_rank());

        let sidebar = match path[1..].rmatch_indices("/").next() {
            Some((idx, _)) => match self.searcher.get_document(&path[1..idx + 1]) {
                Some((f, _)) => tmpl::apply(
                    SIDEBAR,
                    &content!(
                            "parent_dir" => &path[1..idx+1],
                            "current_filename" => &path[idx+1..];
                            "sibling_directories" => f.get_child_directories().iter().map(|s| content!("child" => s, "selected" => s == &path[idx+2..])).collect(),
                            "sibling_files" => f.get_child_files().iter().map(|s| content!("child" => s, "selected" => s == &path[idx+2..])).collect()
                    ),
                ),
                None => {
                    println!("no document for for {}", &path[0..idx + 1]);
                    String::new()
                }
            },
            None => {
                println!("no match for {}", &path[1..]);
                String::new()
            }
        };

        let details = if file.get_is_directory() {
            tmpl::apply(DETAIL_FOLDER, &render::file(&file, content))
        } else if file.get_filename().ends_with(".md") {
            tmpl::apply(
                DETAIL_MD,
                &content!(
                    "markdown" => &markdown::to_html(&content)
                ),
            )
        } else {
            tmpl::apply_with_settings(DETAIL, render::file(&file, content), &self.settings)
        };

        let mut filename_components = Vec::new();
        let mut prev_idx = 0;
        for (idx, _) in file.get_filename().match_indices("/") {
            filename_components.push(content!(
                    "path" => file.get_filename()[0..idx].to_string(),
                    "section" => file.get_filename()[prev_idx..idx].to_string()
            ));
            prev_idx = idx;
        }
        filename_components.push(content!(
                "path" => file.get_filename().to_string(),
                "section" => file.get_filename()[prev_idx..].to_string()
        ));

        let page = tmpl::apply_with_settings(
            DETAIL_TEMPLATE,
            content!(
                "filename" => file.get_filename(),
                "sidebar" => sidebar,
                "detail" => details;

                // Extract the filename into clickable components
                "filename_components" => filename_components
            ),
            &self.settings,
        );

        Response::new(Body::from(self.wrap_template(true, query, page, 0, false)))
    }

    fn index(&self, _path: String, _req: Request) -> Response {
        let page = tmpl::apply_with_settings(INDEX, content!(), &self.settings);
        Response::new(Body::from(self.wrap_template(false, "", page, 0, false)))
    }

    fn not_found(&self, path: String, _req: Request) -> Response {
        Response::new(Body::from(format!("404 not found: path {}", path)))
    }
}

impl Server for SearchWebserver {
    fn respond(&self, path: String, req: Request, token: &str) -> Response {
        if path == "/static/favicon.png" {
            return self.serve_static_file(&path, FAVICON);
        }

        if path.starts_with("/static/") {
            return self.serve_static_files(path, "/static/", &self.static_dir);
        }

        let mut query = String::new();
        let mut is_opensearch = false;

        if let Some(q) = req.uri().query() {
            let params = ws_utils::parse_params(q);
            if let Some(keywords) = params.get("q") {
                // Chrome's search engine plugin turns + into space
                query = keywords.replace("+", " ");
            }

            if params.get("opensearch").is_some() {
                is_opensearch = true;
            }
        };

        if path == "/info" {
            return self.info(&query, req);
        }

        if path == "/opensearch.xml" {
            let output = tmpl::apply(
                OPENSEARCH,
                &content!(
                    "base_url" => &self.base_url
                ),
            );

            return self.serve_static_file(&path, output.as_bytes());
        }

        if path == "/suggest" {
            return self.suggest(&query, is_opensearch, req);
        }

        if path.len() > 1 {
            return self.detail(&query, path, req);
        }

        if query.len() > 0 {
            return self.results(&query, path, req);
        }

        self.index(path, req)
    }
}
