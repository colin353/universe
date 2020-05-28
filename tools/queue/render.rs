use queue_grpc_rust::*;
use server_lib::get_timestamp_usec;
use tmpl;

pub fn artifact(a: &Artifact) -> tmpl::ContentsMap {
    content!(
        "name" => a.get_name(),
        "value" => if !a.get_value_string().is_empty() {
            format!("{}", a.get_value_string())
        } else if a.get_value_bool() {
            String::from("true")
        } else if a.get_value_float() != 0.0 {
            format!("{}", a.get_value_float())
        } else {
            format!("{}", a.get_value_int())
        }
    )
}

fn is_big_artifact(a: &Artifact) -> bool {
    a.get_value_string().len() > 144
}

pub fn message(s: &Message) -> tmpl::ContentsMap {
    let name = if s.get_name().is_empty() {
        format!("{}-{}", s.get_queue(), s.get_id())
    } else {
        s.get_name().to_string()
    };

    let elapsed_time = if s.get_start_time() == 0 {
        0
    } else if s.get_end_time() > 0 {
        s.get_end_time() - s.get_start_time()
    } else {
        get_timestamp_usec() - s.get_start_time()
    };

    let time_in_queue = if s.get_start_time() == 0 {
        get_timestamp_usec() - s.get_enqueued_time()
    } else {
        s.get_start_time() - s.get_enqueued_time()
    };

    content!(
        "id" => s.get_id(),
        "status" => format!("{:?}", s.get_status()),
        "name" => name,
        "enqueued_time" => s.get_enqueued_time(),
        "start_time" => s.get_start_time(),
        "end_time" => s.get_end_time(),
        "elapsed_time" => elapsed_time,
        "time_in_queue" => time_in_queue,
        "queue" => s.get_queue(),
        "failures" => s.get_failures(),
        "reason" => s.get_reason();
        "arguments" => s.get_arguments().iter().map(|a| artifact(a)).collect(),
        "artifacts" => s.get_results().iter().filter(|a| !is_big_artifact(&a)).map(|a| artifact(a)).collect(),
        "big_artifacts" => s.get_results().iter().filter(|a| is_big_artifact(&a)).map(|a| artifact(a)).collect()
    )
}
