extern crate task_client;
extern crate task_lib;
extern crate tasks_grpc_rust;
extern crate tokio;
extern crate weld;

use std::sync::Arc;
use task_lib::{ArtifactBuilder, Task, TaskManager, TaskResultFuture};
use tasks_grpc_rust::{Status, TaskArgument};
use tokio::prelude::Future;

pub const SUBMIT_TASK: &'static str = "submit";
pub const PRESUBMIT_TASK: &'static str = "presubmit";
pub const APPLY_PATCH_TASK: &'static str = "apply_patch";

pub struct WeldSubmitTask {}
impl WeldSubmitTask {
    pub fn new() -> Self {
        Self {}
    }
}
impl Task for WeldSubmitTask {
    fn run(&self, args: &[TaskArgument], manager: Box<dyn TaskManager>) -> TaskResultFuture {
        let status = manager.get_status();
        let passed_args = args.to_owned();
        let passed_status = status.clone();
        let passed_manager = Arc::new(manager);
        let passed_manager_2 = passed_manager.clone();
        let passed_manager_3 = passed_manager.clone();
        Box::new(
            passed_manager
                .spawn(PRESUBMIT_TASK, args.to_owned())
                .and_then(move |s| {
                    if s.get_status() != Status::SUCCESS {
                        return passed_manager_2.failure(status, "query subtask failed");
                    }

                    passed_manager_2.spawn(APPLY_PATCH_TASK, passed_args.to_owned())
                })
                .and_then(move |s| {
                    if s.get_status() != Status::SUCCESS {
                        return passed_manager_3
                            .failure(passed_status, "apply patch subtask failed");
                    }

                    passed_manager_3.success(passed_status)
                }),
        )
    }
}

pub struct ApplyPatchTask {}
impl ApplyPatchTask {
    pub fn new() -> Self {
        Self {}
    }
}
impl Task for ApplyPatchTask {
    fn run(&self, args: &[TaskArgument], manager: Box<dyn TaskManager>) -> TaskResultFuture {
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

        let mut req = weld::ApplyPatchRequest::new();
        req.set_change_id(id as u64);
        let mut response = client.apply_patch(req);

        if !response.get_success() {
            status.mut_artifacts().push(ArtifactBuilder::from_string(
                "reason",
                response.take_reason(),
            ));

            return manager.failure(status, "applying patch failed");
        }
        manager.success(status)
    }
}
