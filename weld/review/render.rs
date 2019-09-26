use tmpl;
use weld;

pub fn file(f: &weld::File) -> tmpl::ContentsMap {
    let content = match std::str::from_utf8(f.get_contents()) {
        Ok(s) => s,
        Err(_) => "<binary data>",
    };
    content!(
        "filename" => f.get_filename(),
        "contents" => base64::encode(content),
        "directory" => f.get_directory()
    )
}

pub fn file_history(fh: &weld::FileHistory) -> tmpl::ContentsMap {
    let mut c = content!(
        "filename" => fh.get_filename();
    );

    let mut is_new_file = true;
    for f in fh.get_snapshots().iter().rev() {
        if f.get_change_id() == 0 {
            continue;
        }

        c.insert("original", file(f));
        is_new_file = false;
    }

    c.insert("status", if is_new_file { "new" } else { "modified" });

    let modified = fh.get_snapshots().last();
    if let Some(f) = modified {
        c.insert("modified", file(f));
    }
    c
}

pub fn change(c: &weld::Change) -> tmpl::ContentsMap {
    content!(
        "id" => format!("{}", c.get_id()),
        "based_index" => format!("{}", c.get_based_index()),
        "friendly_name" => c.get_friendly_name(),
        "submitted_id" => format!("{}", c.get_submitted_id()),
        "author" => c.get_author(),
        "last_modified_timestamp" => c.get_last_modified_timestamp(),
        "description" => c.get_description();
        "changes" => c.get_changes().iter().map(|f| file_history(f)).collect()
    )
}
