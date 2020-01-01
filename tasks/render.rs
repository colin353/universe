use tasks_grpc_rust;
use tmpl;

pub fn argument(a: &tasks_grpc_rust::TaskArgument) -> tmpl::ContentsMap {
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

pub fn artifact(a: &tasks_grpc_rust::TaskArtifact) -> tmpl::ContentsMap {
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

fn is_big_artifact(a: &tasks_grpc_rust::TaskArtifact) -> bool {
    a.get_value_string().len() > 144
}

pub fn status(s: &tasks_grpc_rust::TaskStatus) -> tmpl::ContentsMap {
    content!(
        "id" => s.get_task_id(),
        "status" => format!("{:?}", s.get_status()),
        "name" => s.get_name(),
        "start_time" => s.get_start_time(),
        "end_time" => s.get_end_time(),
        "elapsed_time" => s.get_elapsed_time(),
        "reason" => s.get_reason();
        "arguments" => s.get_arguments().iter().map(|a| argument(a)).collect()
        "artifacts" => s.get_artifacts().iter().filter(|a| !is_big_artifact(&a)).map(|a| artifact(a)).collect()
        "big_artifacts" => s.get_artifacts().iter().filter(|a| is_big_artifact(&a)).map(|a| artifact(a)).collect()
        "subtasks" => s.get_subtasks().iter().map(|s| status(s)).collect()
    )
}
