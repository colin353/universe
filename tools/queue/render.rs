use queue_bus::*;
use server_lib::get_timestamp_usec;
use tmpl;

pub fn artifact(a: &Artifact) -> tmpl::ContentsMap {
    content!(
        "name" => a.name.clone(),
        "value" => if !a.value_string.is_empty() {
            format!("{}", a.value_string)
        } else if a.value_bool {
            String::from("true")
        } else if a.value_float != 0.0 {
            format!("{}", a.value_float)
        } else {
            format!("{}", a.value_int)
        }
    )
}

fn is_big_artifact(a: &Artifact) -> bool {
    a.value_string.len() > 144
}

pub fn message(s: &Message) -> tmpl::ContentsMap {
    let name = if s.name.is_empty() {
        format!("{}-{}", s.queue, s.id)
    } else {
        s.name.to_string()
    };

    let elapsed_time = if s.start_time == 0 {
        0
    } else if s.end_time > 0 {
        s.end_time - s.start_time
    } else {
        get_timestamp_usec() - s.start_time
    };

    let time_in_queue = if s.start_time == 0 {
        get_timestamp_usec() - s.enqueued_time
    } else {
        s.start_time - s.enqueued_time
    };

    content!(
        "id" => s.id,
        "status" => format!("{:?}", s.status),
        "name" => name,
        "enqueued_time" => s.enqueued_time,
        "start_time" => s.start_time,
        "end_time" => s.end_time,
        "elapsed_time" => elapsed_time,
        "time_in_queue" => time_in_queue,
        "queue" => s.queue.clone(),
        "failures" => s.failures,
        "reason" => s.reason.clone();
        "arguments" => s.arguments.iter().map(|a| artifact(a)).collect(),
        "artifacts" => s.results.iter().filter(|a| !is_big_artifact(&a)).map(|a| artifact(a)).collect(),
        "big_artifacts" => s.results.iter().filter(|a| is_big_artifact(&a)).map(|a| artifact(a)).collect()
    )
}
