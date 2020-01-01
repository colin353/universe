extern crate task_client;
extern crate task_lib;
extern crate tasks_grpc_rust;
extern crate tokio;
extern crate weld;

use std::sync::Arc;
use task_lib::{ArtifactBuilder, Task, TaskManager, TaskResultFuture};
use tasks_grpc_rust::{Status, TaskArgument};
use tokio::prelude::{future, stream, Future, Stream};

pub const PRESUBMIT_TASK: &'static str = "presubmit";
pub const QUERY_TASK: &'static str = "query";
pub const BUILD_TASK: &'static str = "build";

pub struct WeldPresubmitTask {}
impl WeldPresubmitTask {
    pub fn new() -> Self {
        Self {}
    }
}
impl Task for WeldPresubmitTask {
    fn run(&self, args: &[TaskArgument], manager: Box<dyn TaskManager>) -> TaskResultFuture {
        let mut status = manager.get_status();
        Box::new(
            manager
                .spawn(QUERY_TASK, args.to_owned())
                .and_then(move |mut s| {
                    if s.get_status() != Status::SUCCESS {
                        return manager.failure(status, "query subtask failed");
                    }

                    // Update the artifacts
                    status.set_artifacts(s.take_artifacts());
                    manager.set_status(&status);

                    let targets: Vec<_> = status
                        .get_artifacts()
                        .iter()
                        .filter_map(|t| {
                            if t.get_name() == "target" || t.get_name() == "dependency" {
                                Some(t.get_value_string().to_owned())
                            } else {
                                None
                            }
                        })
                        .filter(|t| {
                            // We don't want to build/test docker images since those are
                            // time consuming, huge, and extremely unlikely to fail
                            !t.ends_with("_img")
                                && !t.ends_with("_img_push")
                                && !t.ends_with("_img_binary")
                        })
                        .collect();

                    let args = status.get_arguments().to_owned();
                    let passed_manager = Arc::new(manager);
                    let passed_manager_2 = passed_manager.clone();
                    let passed_manager_3 = passed_manager.clone();
                    let passed_status = status.clone();
                    let passed_status_2 = status.clone();
                    Box::new(
                        stream::iter_ok(targets)
                            .for_each(move |target| {
                                let mut b = task_client::ArgumentsBuilder::new();
                                b.add_string("target", target);
                                let mut build_args = args.clone();
                                build_args.append(&mut b.build());
                                passed_manager.spawn(BUILD_TASK, build_args).and_then(|s| {
                                    if s.get_status() != Status::SUCCESS {
                                        return future::err(());
                                    }
                                    future::ok(())
                                })
                            })
                            .and_then(move |()| passed_manager_2.success(passed_status))
                            .or_else(move |()| {
                                passed_manager_3
                                    .failure(passed_status_2, "build/test subtask failed")
                            }),
                    )
                }),
        )
    }
}

pub struct WeldQueryTask {}
impl WeldQueryTask {
    pub fn new() -> Self {
        Self {}
    }
}
impl Task for WeldQueryTask {
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

pub struct WeldBuildTask {}
impl WeldBuildTask {
    pub fn new() -> Self {
        Self {}
    }
}
impl Task for WeldBuildTask {
    fn run(&self, args: &[TaskArgument], manager: Box<dyn TaskManager>) -> TaskResultFuture {
        let mut status = manager.get_status();

        // Validate arguments
        let mut id = 0;
        let mut target = String::new();
        for arg in args {
            if arg.get_name() == "change_id" {
                id = arg.get_value_int();
            }
            if arg.get_name() == "target" {
                target = arg.get_value_string().to_owned();
            }
        }
        if id == 0 {
            return manager.failure(status, "no change_id provided");
        }
        if target.is_empty() {
            return manager.failure(status, "no target provided");
        }

        // Construct weld client
        let config = manager.get_configuration();
        let client = weld::WeldLocalClient::new(&config.weld_hostname, config.weld_port);

        let mut req = weld::RunBuildRequest::new();
        req.set_change_id(id as u64);
        req.set_target(target);
        let mut response = client.run_build(req);

        status.mut_artifacts().push(ArtifactBuilder::from_bool(
            "build_successs",
            response.get_build_success(),
        ));
        status.mut_artifacts().push(ArtifactBuilder::from_string(
            "build_output",
            response.take_build_output(),
        ));
        status.mut_artifacts().push(ArtifactBuilder::from_bool(
            "test_success",
            response.get_test_success(),
        ));
        status.mut_artifacts().push(ArtifactBuilder::from_string(
            "test_output",
            response.take_test_output(),
        ));

        if !response.get_build_success() {
            return manager.failure(status, "build failed");
        }
        if !response.get_test_success() {
            return manager.failure(status, "test failed");
        }

        manager.success(status)
    }
}
