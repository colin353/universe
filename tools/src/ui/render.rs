pub fn change(c: &service::Change) -> tmpl::ContentsMap {
    tmpl::content!(
        "id" => format!("{}", c.id),
        "repo_owner" => c.repo_owner.clone(),
        "repo_name" => c.repo_name.clone(),
        "submitted_id" => format!("{}", c.submitted_id),
        "summary" => c.description.clone(),
        "author" => c.owner.clone(),
        "status" => format!("{:?}", c.status)
    )
}

pub fn file_diff(s: &service::FileDiff) -> tmpl::ContentsMap {
    tmpl::content!(
        "path" => s.path.to_string(),
        "is_dir" => s.is_dir,
        "kind" => format!("{:?}", s.kind)
    )
}

pub fn snapshot(s: &service::Snapshot) -> tmpl::ContentsMap {
    println!("snapshot: {:?}", s);
    tmpl::content!(
        "timestamp" => format!("{:?}", s.timestamp),
        "basis" => core::fmt_basis(s.basis.as_view())
        ;
        "files" => s.files.iter().map(|f| file_diff(f)).collect()
    )
}
