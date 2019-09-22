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

pub fn file_history(f: &weld::FileHistory) -> tmpl::ContentsMap {
    let mut c = content!(
        "filename" => f.get_filename();
    );

    let original = f.get_snapshots().first();
    if let Some(f) = original {
        c.insert("original", file(f));
    }
    let modified = f.get_snapshots().last();
    if let Some(f) = modified {
        c.insert("modified", file(f));
    }
    c
}

pub fn change(c: &weld::Change) -> tmpl::ContentsMap {
    content!(
        "id" => format!("{}", c.get_id()),
        "submitted_id" => format!("{}", c.get_submitted_id()),
        "author" => c.get_author(),
        "description" => c.get_description();
        "changes" => c.get_changes().iter().map(|f| file_history(f)).collect()
    )
}
