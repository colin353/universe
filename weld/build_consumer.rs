use client_service::WeldLocalServiceHandler;
use largetable_client::LargeTableClient;
use lockserv_client::LockservClient;
use queue_client::{
    get_bool_arg, get_int_arg, get_string_arg, ArtifactsBuilder, ConsumeResult, Consumer, Message,
    QueueClient,
};
use weld::{RunBuildQueryRequest, RunBuildRequest};

pub struct BuildConsumer<C: LargeTableClient> {
    queue_client: QueueClient,
    lockserv_client: LockservClient,
    weld: WeldLocalServiceHandler<C>,
}

impl<C: LargeTableClient> BuildConsumer<C> {
    pub fn new(
        weld: WeldLocalServiceHandler<C>,
        queue_client: QueueClient,
        lockserv_client: LockservClient,
    ) -> Self {
        Self {
            weld,
            queue_client,
            lockserv_client,
        }
    }
}

impl<C: LargeTableClient> Consumer for BuildConsumer<C> {
    fn get_queue_client(&self) -> &QueueClient {
        &self.queue_client
    }

    fn get_lockserv_client(&self) -> &LockservClient {
        &self.lockserv_client
    }

    fn consume(&self, message: &Message) -> ConsumeResult {
        let mut outputs = ArtifactsBuilder::new();
        let change_id = match get_int_arg("change", &message) {
            Some(id) => id as u64,
            None => {
                return ConsumeResult::Failure(
                    String::from("must specify argument `change`!"),
                    outputs.build(),
                )
            }
        };

        match get_string_arg("method", &message) {
            Some("query") => {
                let mut req = RunBuildQueryRequest::new();
                req.set_change_id(change_id);
                let mut response = self.weld.run_build_query(&req);

                for target in response.take_targets().into_iter() {
                    outputs.add_string("target", target);
                }

                for dependency in response.take_dependencies().into_iter() {
                    outputs.add_string("dependency", dependency);
                }

                if !response.get_success() {
                    return ConsumeResult::Failure(String::from("build failed"), outputs.build());
                }

                ConsumeResult::Success(outputs.build())
            }
            Some("build") => {
                let mut req = RunBuildRequest::new();
                req.set_change_id(change_id);

                let target = match get_string_arg("target", &message) {
                    Some(t) => t,
                    None => {
                        return ConsumeResult::Failure(
                            String::from("must provide `target` argument"),
                            outputs.build(),
                        )
                    }
                };
                req.set_target(target.to_string());

                let optimized = get_bool_arg("optimized", &message).unwrap_or(false);
                outputs.add_bool("optimized", optimized);
                req.set_optimized(optimized);

                let upload = get_bool_arg("upload", &message).unwrap_or(false);
                outputs.add_bool("upload", upload);
                req.set_upload(upload);

                let is_docker_img_push =
                    get_bool_arg("is_docker_img_push", &message).unwrap_or(false);
                outputs.add_bool("is_docker_img_push", is_docker_img_push);
                req.set_is_docker_img_push(is_docker_img_push);

                let mut response = self.weld.run_build(&req);

                outputs.add_bool("build_success", response.get_build_success());
                outputs.add_string("build_output", response.take_build_output());
                outputs.add_bool("test_success", response.get_test_success());
                outputs.add_string("test_output", response.take_test_output());

                if upload {
                    outputs.add_bool("upload_success", response.get_upload_success());
                    outputs.add_string("upload_output", response.take_upload_output());
                }

                if !response.get_artifact_url().is_empty() {
                    outputs.add_string("artifact_url", response.take_artifact_url());
                }
                if !response.get_docker_img_tag().is_empty() {
                    outputs.add_string("docker_img_tag", response.take_docker_img_tag());
                }

                if !response.get_success() {
                    let reason = if !response.get_build_success() {
                        String::from("build failed")
                    } else if !response.get_test_success() {
                        String::from("tests failed")
                    } else if upload && !response.get_upload_success() {
                        String::from("upload failed")
                    } else {
                        String::from("unknown failure!")
                    };

                    return ConsumeResult::Failure(reason, outputs.build());
                }

                ConsumeResult::Success(outputs.build())
            }
            Some(method) => {
                ConsumeResult::Failure(format!("unknown method: `{}`", method), outputs.build())
            }
            None => ConsumeResult::Failure(
                String::from("must provide `method` argument"),
                outputs.build(),
            ),
        }
    }
}
