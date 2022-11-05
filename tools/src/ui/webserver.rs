extern crate tmpl;

static MODIFIED_FILES: &str = include_str!("modified_files.html");
static DIFF_VIEW: &str = include_str!("diff_view.html");
static TEMPLATE: &str = include_str!("template.html");
static CHANGE: &str = include_str!("change.html");
static INDEX: &str = include_str!("homepage.html");

#[derive(Clone)]
pub struct SrcUIServer {}

impl SrcUIServer {
    pub fn new() -> Self {
        Self {}
    }

    fn wrap_template(&self, content: String) -> String {
        tmpl::apply(
            TEMPLATE,
            &tmpl::content!(
                "title" => "src",
                "content" => content
            ),
        )
    }

    fn index(&self, _path: String, _req: Request) -> Response {
        let changes = self
            .client
            .list_changes()
            .iter()
            .rev()
            .filter(|c| c.get_status() != weld::ChangeStatus::ARCHIVED)
            .map(|c| render::change(c))
            .collect::<Vec<_>>();

        let mut req = weld::GetSubmittedChangesRequest::new();
        req.set_limit(15);
        let submitted_changes = self
            .client
            .get_submitted_changes(req)
            .iter()
            .map(|c| render::change(c))
            .collect::<Vec<_>>();

        let page = tmpl::apply(
            INDEX,
            &content!(;
                "progress" => changes,
                "submitted" => submitted_changes
            ),
        );

        Response::new(Body::from(self.wrap_template(page)))
    }
}
