extern crate task_lib;
extern crate tasks_grpc_rust;
extern crate tokio;
extern crate weld;

use task_lib::{ArtifactBuilder, Task, TaskManager, TaskResultFuture};
use tasks_grpc_rust::{Status, TaskArgument};
use tokio::prelude::{future, Future};

pub struct WeldBuildTask {}
impl WeldBuildTask {
    pub fn new() -> Self {
        Self {}
    }
}
impl Task for WeldBuildTask {
    fn run(&self, args: &[TaskArgument], manager: Box<dyn TaskManager>) -> TaskResultFuture {
        println!("run build");
        let mut status = manager.get_status();

        // Validate arguments
        let mut id = 0;
        for arg in args {
            if arg.get_name() == "change_id" {
                id = arg.get_value_int()
            }
        }
        if id == 0 {
            return manager.failure(status, "no change_id provided");
        }

        // Construct weld client
        let config = manager.get_configuration();
        let client = weld::WeldLocalClient::new(&config.weld_hostname, config.weld_port);

        let mut req = weld::RunBuildQueryRequest::new();
        req.set_change_id(id as u64);
        let mut response = client.run_build_query(req);

        for target in response.take_targets().into_iter() {
            status
                .mut_artifacts()
                .push(ArtifactBuilder::from_string("target", target))
        }

        for target in response.take_dependencies().into_iter() {
            status
                .mut_artifacts()
                .push(ArtifactBuilder::from_string("dependency", target))
        }

        if !response.get_success() {
            return manager.failure(status, "bazel query failed");
        }
        manager.success(status)
    }
}
