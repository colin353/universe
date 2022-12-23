pub fn change(c: &service::Change) -> tmpl::ContentsMap {
    tmpl::content!(
        "id" => format!("{}", c.id),
        "repo_owner" => c.repo_owner.clone(),
        "repo_name" => c.repo_name.clone(),
        "submitted_id" => format!("{}", c.submitted_id),
        // TODO: parse the summary out of the description
        "summary" => c.description.clone(),
        "description" => c.description.clone(),
        "author" => c.owner.clone(),
        "status" => format!("{:?}", c.status);
        "tasks" => vec![]
    )
}

pub fn file_diff(s: &service::FileDiff) -> tmpl::ContentsMap {
    tmpl::content!(
        "path" => s.path.to_string(),
        "language" => format!("{:?}", language_specific::get_filetype(&s.path)).to_lowercase(),
        "is_dir" => s.is_dir,
        "kind" => format!("{:?}", s.kind)
    )
}

pub fn file_history(
    s: &service::FileDiff,
    original: Vec<u8>,
    modified: Vec<u8>,
) -> tmpl::ContentsMap {
    let mut original_is_binary = false;
    let original_s = match String::from_utf8(original) {
        Ok(s) => s,
        Err(_) => {
            original_is_binary = true;
            String::new()
        }
    };

    let mut modified_is_binary = false;
    let modified_s = match String::from_utf8(modified) {
        Ok(s) => s,
        Err(_) => {
            modified_is_binary = true;
            String::new()
        }
    };

    tmpl::content!(
        "path" => s.path.to_string(),
        "language" => format!("{:?}", language_specific::get_filetype(&s.path)).to_lowercase(),
        "original_is_binary" => original_is_binary,
        "original" => base64::encode(&original_s),
        "modified" => base64::encode(&modified_s),
        "is_dir" => s.is_dir,
        "kind" => format!("{:?}", s.kind)
    )
}

pub fn snapshot(s: &service::Snapshot) -> tmpl::ContentsMap {
    let basis = core::fmt_basis(s.basis.as_view());
    let short_basis = basis.split("/").skip(3).collect::<Vec<_>>().join("/");

    tmpl::content!(
        "timestamp" => format!("{:?}", s.timestamp),
        "basis" => core::fmt_basis(s.basis.as_view()),
        "short_basis" => short_basis
        ;
        "files" => s.files.iter().map(|f| file_diff(f)).collect()
    )
}
