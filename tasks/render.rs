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

pub fn status(s: &tasks_grpc_rust::TaskStatus) -> tmpl::ContentsMap {
    content!(
        "id" => s.get_task_id(),
        "status" => format!("{:?}", s.get_status()),
        "name" => s.get_name(),
        "reason" => s.get_reason();
        "arguments" => s.get_arguments().iter().map(|a| argument(a)).collect()
        "artifacts" => s.get_artifacts().iter().map(|a| artifact(a)).collect()
    )
}
