pub fn change(c: &service::Change) -> tmpl::ContentsMap {
    tmpl::content!(
        "id" => format!("{}", c.id),
        "submitted_id" => format!("{}", c.submitted_id),
        "summary" => format!("{}", c.description),
        "author" => format!("{}", c.owner),
        "status" => format!("{:?}", c.status)
    )
}
