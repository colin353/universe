extern crate task_client;
extern crate task_lib;
extern crate tasks_grpc_rust;
extern crate tokio;
extern crate weld;
extern crate x20_client;
extern crate x20_grpc_rust as x20;

use std::sync::Arc;
use task_lib::{ArtifactBuilder, Task, TaskManager, TaskResultFuture};
use tasks_grpc_rust::{Status, TaskArgument};
use tokio::prelude::{future, stream, Future, Stream};

pub const PUBLISH_TASK: &'static str = "x20_publish";
pub const QUERY_TASK: &'static str = "x20_query";

pub struct X20QueryTask {}
impl X20QueryTask {
    pub fn new() -> Self {
        Self {}
    }
}
impl Task for X20QueryTask {
    fn run(&self, args: &[TaskArgument], manager: Box<dyn TaskManager>) -> TaskResultFuture {
        let status = manager.get_status();
        let targets: std::collections::HashSet<_> = args
            .iter()
            .filter(|arg| arg.get_name() == "target")
            .map(|arg| arg.get_value_string().to_owned())
            .collect();

        let client = {
            let config = manager.get_configuration();
            x20_client::X20Client::new(&config.x20_hostname, config.x20_port)
        };

        let binaries_to_rebuild: Vec<_> = client
            .get_binaries()
            .into_iter()
            .filter(|bin| !bin.get_target().is_empty() && targets.contains(bin.get_target()))
            .map(|mut bin| bin.take_name())
            .collect();

        let passed_manager = Arc::new(manager);
        let passed_manager_2 = passed_manager.clone();
        let passed_manager_3 = passed_manager.clone();
        let passed_status = status.clone();
        let passed_status_2 = status.clone();
        let mut build_args = args.to_vec();
        Box::new(
            stream::iter_ok(binaries_to_rebuild)
                .for_each(move |binary| {
                    let mut b = task_client::ArgumentsBuilder::new();
                    b.add_string("binary", binary);
                    build_args.append(&mut b.build());
                    passed_manager
                        .spawn(PUBLISH_TASK, build_args.clone())
                        .and_then(|s| {
                            if s.get_status() != Status::SUCCESS {
                                return future::err(());
                            }
                            future::ok(())
                        })
                })
                .and_then(move |()| passed_manager_2.success(passed_status))
                .or_else(move |()| {
                    passed_manager_3.failure(passed_status_2, "publish subtask failed!")
                }),
        )
    }
}

pub struct X20PublishTask {}
impl X20PublishTask {
    pub fn new() -> Self {
        Self {}
    }
}
impl Task for X20PublishTask {
    fn run(&self, args: &[TaskArgument], manager: Box<dyn TaskManager>) -> TaskResultFuture {
        let mut status = manager.get_status();

        // Validate arguments
        let mut id = 0;
        let mut binary_name = String::new();
        for arg in args {
            if arg.get_name() == "change_id" {
                id = arg.get_value_int();
            }
            if arg.get_name() == "binary" {
                binary_name = arg.get_value_string().to_owned();
            }
        }
        if id == 0 {
            return manager.failure(status, "no change_id provided");
        }
        if binary_name.is_empty() {
            return manager.failure(status, "no binary provided");
        }

        let x20_client = {
            let config = manager.get_configuration();
            x20_client::X20Client::new(&config.x20_hostname, config.x20_port)
        };

        let binary = match x20_client
            .get_binaries()
            .into_iter()
            .filter(|bin| bin.get_name() == &binary_name)
            .next()
        {
            Some(b) => b,
            None => {
                return manager
                    .failure(status, &format!("could not find binary `{}`", binary_name));
            }
        };

        if binary.get_target().is_empty() {
            return manager.failure(status, "binary has no target! cannot rebuild");
        }

        let client = {
            let config = manager.get_configuration();
            weld::WeldLocalClient::new(&config.weld_client_hostname, config.weld_client_port)
        };
        let mut req = weld::RunBuildRequest::new();
        req.set_target(binary.get_target().to_owned());
        req.set_optimized(true);
        req.set_upload(true);

        if !binary.get_docker_img().is_empty() {
            req.set_is_docker_img_push(true);
        }

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
        status.mut_artifacts().push(ArtifactBuilder::from_bool(
            "upload_success",
            response.get_upload_success(),
        ));
        status.mut_artifacts().push(ArtifactBuilder::from_string(
            "upload_output",
            response.take_upload_output(),
        ));

        if !binary.get_docker_img().is_empty() {
            status.mut_artifacts().push(ArtifactBuilder::from_string(
                "docker_img",
                binary.get_docker_img().to_owned(),
            ));
            status.mut_artifacts().push(ArtifactBuilder::from_string(
                "docker_img_tag",
                response.get_docker_img_tag().to_owned(),
            ));
        } else {
            status.mut_artifacts().push(ArtifactBuilder::from_string(
                "artifact_url",
                response.get_artifact_url().to_owned(),
            ));
        }

        if !response.get_build_success() {
            return manager.failure(status, "build failed");
        }
        if !response.get_test_success() {
            return manager.failure(status, "test failed");
        }
        if !response.get_upload_success() {
            return manager.failure(status, "upload failed");
        }

        // Publish the binary to x20
        let mut req = x20::PublishBinaryRequest::new();
        *req.mut_binary() = binary;
        req.mut_binary()
            .set_url(response.get_artifact_url().to_owned());
        req.mut_binary()
            .set_docker_img_tag(response.get_docker_img_tag().to_owned());
        x20_client.publish_binary(req);

        manager.success(status)
    }
}
