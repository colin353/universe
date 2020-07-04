use largetable_client::LargeTableClient;
use lockserv_client::LockservClient;
use queue_client::*;
use x20_server_lib::X20ServiceHandler;

use std::collections::HashMap;

pub struct X20Consumer<C: largetable_client::LargeTableClient> {
    queue_client: QueueClient,
    lockserv_client: lockserv_client::LockservClient,
    x20_client: X20ServiceHandler<C>,
}

impl<C: LargeTableClient> X20Consumer<C> {
    pub fn new(
        queue_client: QueueClient,
        lockserv_client: lockserv_client::LockservClient,
        x20_client: X20ServiceHandler<C>,
    ) -> Self {
        Self {
            queue_client,
            lockserv_client,
            x20_client,
        }
    }
}

impl<C: LargeTableClient + Clone> Consumer for X20Consumer<C> {
    fn get_queue_client(&self) -> &QueueClient {
        &self.queue_client
    }

    fn get_lockserv_client(&self) -> &LockservClient {
        &self.lockserv_client
    }

    fn consume(&self, message: &Message) -> ConsumeResult {
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

        let binaries = self.x20_client.get_binaries().take_binaries();
        let mut binmap = HashMap::new();
        for binary in binaries.iter() {
            binmap.insert(binary.get_target().to_string(), binary);
        }

        let mut blockers = Vec::new();
        for arg in message.get_arguments() {
            if arg.get_name() == "target" {
                let target = arg.get_value_string();
                if let Some(bin) = binmap.get(target) {
                    // We found a target to rebuild, so let's enqueue a build for it
                    let mut m = Message::new();
                    m.set_name(format!("build + publish {}", target));

                    let mut args = ArtifactsBuilder::new();
                    args.add_string("method", String::from("build"));
                    args.add_string("target", target.to_string());
                    args.add_int("change", change_id);
                    args.add_bool("upload", true);
                    args.add_bool("optimized", true);
                    args.add_bool("is_submitted", true);
                    args.add_bool("is_docker_img_push", !bin.get_docker_img_tag().is_empty());
                    *m.mut_arguments() = args.build_rf();

                    let id = self.get_queue_client().enqueue(String::from("builds"), m);

                    let mut b = BlockingMessage::new();
                    b.set_queue(String::from("builds"));
                    b.set_id(id);
                    blockers.push(b);
                }
            }
        }

        if blockers.len() == 0 {
            return ConsumeResult::Success(outputs.build());
        }

        ConsumeResult::Blocked(outputs.build(), blockers)
    }

    fn resume(&self, message: &Message) -> ConsumeResult {
        let mut outputs = ArtifactsBuilder::new();

        // Rebuild the binary map
        let binaries = self.x20_client.get_binaries().take_binaries();
        let mut binmap = HashMap::new();
        for binary in binaries.iter() {
            binmap.insert(binary.get_target().to_string(), binary);
        }

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

            let target = match get_string_arg("target", &m) {
                Some(t) => t,
                None => {
                    return ConsumeResult::Failure(
                        format!("no target for subtask!"),
                        outputs.build(),
                    );
                }
            };

            let url = match get_string_arg("artifact_url", &m) {
                Some(t) => t.to_string(),
                None => String::new(),
            };

            let img_tag = match get_string_arg("docker_img_tag", &m) {
                Some(t) => t.to_string(),
                None => String::new(),
            };

            // Actually publish the resulting artifacts
            let mut req = x20_grpc_rust::PublishBinaryRequest::new();
            *req.mut_binary() = match binmap.get(target) {
                Some(b) => (*b).clone(),
                None => {
                    return ConsumeResult::Failure(
                        format!("no corresponding binary for {}", target),
                        outputs.build(),
                    );
                }
            };
            if !url.is_empty() {
                req.mut_binary().set_url(url);
            }
            if !img_tag.is_empty() {
                req.mut_binary().set_docker_img_tag(img_tag);
            }

            let response = self.x20_client.publish_binary(req, false);
            if response.get_error() != x20_grpc_rust::Error::NONE {
                return ConsumeResult::Failure(
                    format!(
                        "failed to publish binary {}, error {:?}",
                        target,
                        response.get_error()
                    ),
                    outputs.build(),
                );
            }
        }

        ConsumeResult::Success(outputs.build())
    }
}
