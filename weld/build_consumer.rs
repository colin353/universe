use client_service::WeldLocalServiceHandler;
use largetable_client::LargeTableClient;
use lockserv_client::LockservClient;
use queue_client::{
    get_bool_arg, get_int_arg, get_string_arg, ArtifactsBuilder, BlockingMessage, ConsumeResult,
    Consumer, Message, QueueClient, Status,
};
use weld::{RunBuildQueryRequest, RunBuildRequest, WeldServer};

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
                req.set_is_submitted(get_bool_arg("is_submitted", &message).unwrap_or(false));

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

pub struct PresubmitConsumer {
    queue_client: QueueClient,
    lockserv_client: LockservClient,
}

impl PresubmitConsumer {
    pub fn new(queue_client: QueueClient, lockserv_client: LockservClient) -> Self {
        Self {
            queue_client,
            lockserv_client,
        }
    }
}

impl Consumer for PresubmitConsumer {
    fn get_queue_client(&self) -> &QueueClient {
        &self.queue_client
    }

    fn get_lockserv_client(&self) -> &LockservClient {
        &self.lockserv_client
    }

    fn consume(&self, message: &Message) -> ConsumeResult {
        // First stage: schedule query
        let mut outputs = ArtifactsBuilder::new();

        let change_id = match get_int_arg("change", &message) {
            Some(0) => {
                return ConsumeResult::Failure(
                    String::from("no `change` provided"),
                    outputs.build(),
                )
            }
            Some(id) => id as i64,
            None => {
                return ConsumeResult::Failure(
                    String::from("must specify argument `change`!"),
                    outputs.build(),
                )
            }
        };

        let mut msg = Message::new();
        msg.set_name(format!("query c/{}", change_id));
        let mut args = ArtifactsBuilder::new();
        args.add_string("method", String::from("query"));
        args.add_int("change", change_id);

        for arg in args.build() {
            msg.mut_arguments().push(arg);
        }
        msg.mut_blocks().set_id(message.get_id());
        msg.mut_blocks().set_queue(message.get_queue().to_string());

        let id = self.get_queue_client().enqueue(String::from("builds"), msg);

        let mut blocker = BlockingMessage::new();
        blocker.set_id(id);
        blocker.set_queue(String::from("builds"));

        ConsumeResult::Blocked(outputs.build(), vec![blocker])
    }

    fn resume(&self, message: &Message) -> ConsumeResult {
        let mut outputs = ArtifactsBuilder::new();
        if message.get_blocked_by().len() == 1 {
            // We're coming back from a query, and need to schedule builds of targets/deps
            let id = message.get_blocked_by()[0].get_id();
            let queue = message.get_blocked_by()[0].get_queue();

            let m = match self.get_queue_client().read(queue.to_string(), id) {
                Some(m) => m,
                None => {
                    return ConsumeResult::Failure(
                        String::from("unable to read blocking task"),
                        outputs.build(),
                    )
                }
            };

            if m.get_status() != Status::SUCCESS {
                return ConsumeResult::Failure(String::from("query task failed!"), outputs.build());
            }

            let mut blockers = Vec::new();
            for target in m
                .get_results()
                .iter()
                .filter(|r| r.get_name() == "dependency" || r.get_name() == "target")
            {
                let build_target = target.get_value_string();
                outputs.add_string(target.get_name(), build_target.to_string());

                // Docker image targets are not productive to build, so skip those.
                if build_target.ends_with("_img")
                    || build_target.ends_with("_img_push")
                    || build_target.ends_with("_img_binary")
                {
                    continue;
                }

                let mut args = ArtifactsBuilder::new();

                let change_id = get_int_arg("change", &message).unwrap();
                args.add_int("change", change_id);
                args.add_string("method", "build".to_string());
                args.add_string("target", build_target.to_string());

                let mut m = Message::new();
                m.set_name(format!("build + test {}", build_target));
                for arg in args.build() {
                    m.mut_arguments().push(arg);
                }
                m.mut_blocks().set_id(message.get_id());
                m.mut_blocks().set_queue(message.get_queue().to_string());

                let id = self.get_queue_client().enqueue(String::from("builds"), m);

                let mut b = BlockingMessage::new();
                b.set_queue(String::from("builds"));
                b.set_id(id);
                blockers.push(b);
            }

            if blockers.len() > 0 {
                ConsumeResult::Blocked(outputs.build(), blockers)
            } else {
                ConsumeResult::Success(outputs.build())
            }
        } else {
            // All builds are done, so we just need to check for success.
            for blocker in message.get_blocked_by() {
                let m = match self
                    .get_queue_client()
                    .read(blocker.get_queue().to_string(), blocker.get_id())
                {
                    Some(m) => m,
                    None => {
                        return ConsumeResult::Failure(
                            String::from("must specify argument `change`!"),
                            outputs.build(),
                        )
                    }
                };

                if m.get_status() != Status::SUCCESS {
                    return ConsumeResult::Failure(String::from("build failed"), outputs.build());
                }
            }

            ConsumeResult::Success(outputs.build())
        }
    }
}

pub struct SubmitConsumer<C: LargeTableClient> {
    queue_client: QueueClient,
    lockserv_client: LockservClient,
    weld_client: WeldLocalServiceHandler<C>,
    weld_server: weld::WeldServerClient,
}

impl<C: LargeTableClient> SubmitConsumer<C> {
    pub fn new(
        weld_client: WeldLocalServiceHandler<C>,
        weld_server: weld::WeldServerClient,
        queue_client: QueueClient,
        lockserv_client: LockservClient,
    ) -> Self {
        Self {
            weld_server,
            weld_client,
            queue_client,
            lockserv_client,
        }
    }
}

impl<C: LargeTableClient> Consumer for SubmitConsumer<C> {
    fn get_queue_client(&self) -> &QueueClient {
        &self.queue_client
    }

    fn get_lockserv_client(&self) -> &LockservClient {
        &self.lockserv_client
    }

    fn consume(&self, message: &Message) -> ConsumeResult {
        // First, run presubmit tests
        let mut outputs = ArtifactsBuilder::new();
        let change_id = match get_int_arg("change", &message) {
            Some(0) => {
                return ConsumeResult::Failure(
                    String::from("no `change` provided"),
                    outputs.build(),
                )
            }
            Some(id) => id as i64,
            None => {
                return ConsumeResult::Failure(
                    String::from("must specify argument `change`!"),
                    outputs.build(),
                )
            }
        };

        // First, double check that the change is valid and synced
        let mut c = weld::Change::new();
        c.set_id(change_id as u64);
        let change = self.weld_server.get_change(c);
        if !change.get_found() {
            return ConsumeResult::Failure(
                format!("change {} does not exist!", change_id),
                outputs.build(),
            );
        }

        let most_recent_change = self.weld_server.get_latest_change();
        if change.get_based_index() != most_recent_change.get_submitted_id() {
            return ConsumeResult::Failure(
                "change out of date, requires sync".to_string(),
                outputs.build(),
            );
        }

        // OK, we are good to submit, so
        let mut args = ArtifactsBuilder::new();
        args.add_int("change", change_id);

        let mut m = Message::new();
        m.set_name(format!("presubmit for c/{}", change_id));
        for arg in args.build() {
            m.mut_arguments().push(arg);
        }
        m.mut_blocks().set_id(message.get_id());
        m.mut_blocks().set_queue(message.get_queue().to_string());

        let id = self
            .get_queue_client()
            .enqueue(String::from("presubmit"), m);

        let mut b = BlockingMessage::new();
        b.set_queue(String::from("presubmit"));
        b.set_id(id);

        ConsumeResult::Blocked(outputs.build(), vec![b])
    }

    fn resume(&self, message: &Message) -> ConsumeResult {
        let mut outputs = ArtifactsBuilder::new();

        let mut targets = std::collections::HashSet::new();

        // Check that presubmit has passed
        for blocker in message.get_blocked_by() {
            let m = match self
                .get_queue_client()
                .read(blocker.get_queue().to_string(), blocker.get_id())
            {
                Some(m) => m,
                None => {
                    return ConsumeResult::Failure(
                        String::from("must specify argument `change`!"),
                        outputs.build(),
                    )
                }
            };

            if m.get_status() != Status::SUCCESS {
                return ConsumeResult::Failure(String::from("build failed"), outputs.build());
            }

            for result in m.get_results() {
                if result.get_name() == "target" || result.get_name() == "dependency" {
                    targets.insert(result.get_value_string().to_string());
                }
            }
        }

        // Then apply the patch to the backup git server
        let change_id = get_int_arg("change", &message).unwrap();
        let mut req = weld::ApplyPatchRequest::new();
        req.set_change_id(change_id as u64);
        let mut response = self.weld_client.apply_patch(req);

        if !response.get_success() {
            return ConsumeResult::Failure(response.take_reason(), outputs.build());
        }

        // And finally merge the actual change
        let mut change = weld::Change::new();
        change.set_remote_id(change_id as u64);
        let mut response = self.weld_client.submit(change);

        if response.get_status() != weld::SubmitStatus::OK {
            return ConsumeResult::Failure(
                format!("submit failed: {:?}", response.get_status()),
                outputs.build(),
            );
        }

        // Submit is a success, so let's also schedule a publish task to deploy
        // these binaries
        let mut args = ArtifactsBuilder::new();
        args.add_int("change", change_id);
        for target in targets.into_iter() {
            args.add_string("target", target);
        }

        let mut m = Message::new();
        m.set_name(format!("publish c/{}", change_id));
        *m.mut_arguments() = args.build_rf();
        self.get_queue_client().enqueue(String::from("publish"), m);

        ConsumeResult::Success(outputs.build())
    }
}
