use task_client;
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

pub fn file_history(fh: &weld::FileHistory, index: u64) -> Option<tmpl::ContentsMap> {
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
        break;
    }

    c.insert("status", if is_new_file { "new" } else { "modified" });

    let mut has_file = false;
    if index == 0 {
        if let Some(f) = fh.get_snapshots().last() {
            has_file = true;
            c.insert("modified", file(f));
        }
    } else {
        for f in fh.get_snapshots().iter().rev() {
            if f.get_snapshot_id() != index {
                continue;
            }

            has_file = true;
            c.insert("modified", file(f));
        }
    }

    if has_file {
        Some(c)
    } else {
        None
    }
}

pub fn change(c: &weld::Change) -> tmpl::ContentsMap {
    // Figure out what the latest snapshot is
    let mut latest_snapshot = 0;
    for change in c.get_changes() {
        for snapshot in change.get_snapshots() {
            if snapshot.get_snapshot_id() > latest_snapshot {
                latest_snapshot = snapshot.get_snapshot_id();
            }
        }
    }

    content!(
        "id" => format!("{}", c.get_id()),
        "based_index" => format!("{}", c.get_based_index()),
        "friendly_name" => c.get_friendly_name(),
        "submitted_id" => format!("{}", c.get_submitted_id()),
        "author" => c.get_author(),
        "last_modified_timestamp" => c.get_last_modified_timestamp(),
        "summary" => weld::summarize_change_description(c.get_description()),
        "description" => weld::render_change_description(c.get_description());
        "changes" => c.get_changes().iter().filter_map(|f| file_history(f, latest_snapshot)).collect()
    )
}

pub fn get_task_pills(c: &task_client::TaskStatus) -> Vec<tmpl::ContentsMap> {
    if c.get_subtasks().len() == 0 {
        return vec![content!(
            "name" => c.get_name(),
            "status" => format!("{:?}", c.get_status())
        )];
    }

    c.get_subtasks()
        .iter()
        .map(|x| get_task_pills(x))
        .flatten()
        .collect()
}
