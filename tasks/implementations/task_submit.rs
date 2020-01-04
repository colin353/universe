extern crate task_client;
extern crate task_lib;
extern crate tasks_grpc_rust;
extern crate tokio;
extern crate weld;

use std::sync::Arc;
use task_lib::{ArtifactBuilder, Task, TaskManager, TaskResultFuture};
use tasks_grpc_rust::{Status, TaskArgument};
use tokio::prelude::Future;
use weld::WeldServer;

pub const TRY_SUBMIT_TASK: &'static str = "try_submit";
pub const SUBMIT_TASK: &'static str = "submit";
pub const PRESUBMIT_TASK: &'static str = "presubmit";
pub const APPLY_PATCH_TASK: &'static str = "apply_patch";

pub struct WeldTrySubmitTask {}
impl WeldTrySubmitTask {
    pub fn new() -> Self {
        Self {}
    }
}
impl Task for WeldTrySubmitTask {
    fn run(&self, args: &[TaskArgument], manager: Box<dyn TaskManager>) -> TaskResultFuture {
        let status = manager.get_status();

        // Before starting the submit, let's check whether our change is up to date
        let client = {
            let config = manager.get_configuration();
            weld::WeldServerClient::new(
                &config.weld_server_hostname,
                String::new(),
                config.weld_server_port,
            )
        };

        let mut id = 0;
        for arg in args {
            if arg.get_name() == "change_id" {
                id = arg.get_value_int()
            }
        }
        if id == 0 {
            return manager.failure(status, "no change_id provided");
        }

        let mut c = weld::Change::new();
        c.set_id(id as u64);
        let change = client.get_change(c);
        if !change.get_found() {
            return manager.failure(status, "could not find change");
        }

        let most_recent_change = client.get_latest_change();
        if change.get_based_index() != most_recent_change.get_submitted_id() {
            return manager.failure(status, "change out of date, requires sync");
        }

        let passed_args = args.to_owned();
        let passed_args_2 = args.to_owned();
        let passed_status = status.clone();
        let passed_status_2 = status.clone();
        let passed_manager = Arc::new(manager);
        let passed_manager_2 = passed_manager.clone();
        let passed_manager_3 = passed_manager.clone();
        let passed_manager_4 = passed_manager.clone();
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
                        return passed_manager_3.failure(passed_status, "patch subtask failed");
                    }

                    passed_manager_3.spawn(SUBMIT_TASK, passed_args_2.to_owned())
                })
                .and_then(move |s| {
                    if s.get_status() != Status::SUCCESS {
                        return passed_manager_4
                            .failure(passed_status_2, "apply patch subtask failed");
                    }

                    passed_manager_4.success(passed_status_2)
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
        let client = {
            let config = manager.get_configuration();
            weld::WeldLocalClient::new(&config.weld_client_hostname, config.weld_client_port)
        };

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

pub struct SubmitTask {}
impl SubmitTask {
    pub fn new() -> Self {
        Self {}
    }
}
impl Task for SubmitTask {
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
        let client = weld::WeldServerClient::new(
            &config.weld_server_hostname,
            String::new(),
            config.weld_server_port,
        );

        let mut req = weld::Change::new();
        req.set_id(id as u64);
        let mut response = client.submit(req);

        if response.get_status() != weld::SubmitStatus::OK {
            status.mut_artifacts().push(ArtifactBuilder::from_string(
                "reason",
                format!("submit failed: {:?}", response.get_status()),
            ));

            return manager.failure(status, "submit failed");
        }
        manager.success(status)
    }
}
