use tasks_grpc_rust;
use tmpl;

pub fn status(s: &tasks_grpc_rust::TaskStatus) -> tmpl::ContentsMap {
    content!(
        "id" => s.get_task_id(),
        "status" => format!("{:?}", s.get_status()),
        "name" => s.get_name()
    )
}
