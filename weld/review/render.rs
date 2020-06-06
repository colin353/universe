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
    let mut is_deleted = true;
    for f in fh.get_snapshots().iter().rev() {
        if f.get_change_id() == 0 {
            continue;
        }

        if f.get_deleted() {
            is_deleted = true;
        }

        c.insert("original", file(f));
        is_new_file = false;
        break;
    }

    let status = match (is_new_file, is_deleted) {
        (true, true) => "deleted",
        (true, false) => "new",
        _ => "modified",
    };
    c.insert("status", status);

    let mut has_file = false;
    let mut is_directory = false;
    if index == 0 {
        if let Some(f) = fh.get_snapshots().last() {
            has_file = true;
            c.insert("modified", file(f));
            is_directory = f.get_directory();
        }
    } else {
        for f in fh.get_snapshots().iter().rev() {
            if f.get_snapshot_id() != index {
                continue;
            }

            has_file = true;
            c.insert("modified", file(f));
            is_directory = f.get_directory();
        }
    }

    c.insert("directory", is_directory);

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
        "status" => format!("{:?}", c.get_status()),
        "last_modified_timestamp" => c.get_last_modified_timestamp(),
        "summary" => weld::summarize_change_description(c.get_description()),
        "description" => weld::render_change_description(c.get_description());
        "changes" => c.get_changes().iter().filter_map(|f| file_history(f, latest_snapshot)).collect()
    )
}

pub fn get_task_pills(c: &queue_client::Message) -> tmpl::ContentsMap {
    return content!(
        "name" => c.get_name(),
        "status" => format!("{:?}", c.get_status()),
        "info_url" => c.get_info_url()
    );
}
