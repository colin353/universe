use futures::join;
use lockserv_client::LockservClient;
use queue_client::Consumer;
use queue_client::{
    ArtifactsBuilder, BlockingMessage, ConsumeResult, Message, QueueClient, Status,
};
use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;

#[tokio::main]
async fn main() {
    let q = queue_client::QueueClient::new_metal("queue.bus");
    let ls = lockserv_client::LockservClient::new_metal("lockserv.bus");

    let mut args = ArtifactsBuilder::new();
    args.add_string("basis", "src.colinmerkel.xyz/colin/code/3".to_string());
    q.enqueue(
        "presubmit".to_string(),
        Message {
            name: "build colin/code/3".to_string(),
            arguments: args.build(),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    let presubmit_consumer = PresubmitConsumer {
        queue_client: q.clone(),
        lockserv_client: ls.clone(),
    };

    let builds_consumer = BuildConsumer {
        queue_client: q.clone(),
        lockserv_client: ls.clone(),
    };

    futures::join!(
        presubmit_consumer.start("presubmit".to_string()),
        builds_consumer.start("builds".to_string())
    );
}

#[derive(Clone)]
struct BuildConsumer {
    queue_client: QueueClient,
    lockserv_client: LockservClient,
}

impl queue_client::Consumer for BuildConsumer {
    fn get_queue_client(&self) -> &QueueClient {
        &self.queue_client
    }

    fn get_lockserv_client(&self) -> &LockservClient {
        &self.lockserv_client
    }

    fn consume(&self, message: &Message) -> Pin<Box<dyn Future<Output = ConsumeResult> + Send>> {
        let message = message.clone();
        Box::pin(async move { build(message).await })
    }
}

#[derive(Clone)]
struct PresubmitConsumer {
    queue_client: QueueClient,
    lockserv_client: LockservClient,
}

impl queue_client::Consumer for PresubmitConsumer {
    fn get_queue_client(&self) -> &QueueClient {
        &self.queue_client
    }

    fn get_lockserv_client(&self) -> &LockservClient {
        &self.lockserv_client
    }

    fn resume(&self, message: &Message) -> Pin<Box<dyn Future<Output = ConsumeResult> + Send>> {
        let mut output = ArtifactsBuilder::new();

        let _self = self.clone();
        let message = message.clone();
        Box::pin(async move {
            let client = _self.get_queue_client();
            for blocker in &message.blocked_by {
                let m = match client.read(blocker.queue.clone(), blocker.id).await {
                    Ok(Some(m)) => m,
                    Ok(None) => {
                        return ConsumeResult::Failure(
                            "blocking task not found!".to_string(),
                            Vec::new(),
                        )
                    }
                    Err(e) => {
                        return ConsumeResult::Failure(
                            "failed to read blocking task due to RPC error!".to_string(),
                            Vec::new(),
                        )
                    }
                };

                if m.status != Status::Success {
                    return ConsumeResult::Failure("build failed".to_string(), Vec::new());
                }
            }

            ConsumeResult::Success(Vec::new())
        })
    }

    fn consume(&self, message: &Message) -> Pin<Box<dyn Future<Output = ConsumeResult> + Send>> {
        println!("picked up presubmit task");
        let _self = self.clone();
        let message = message.clone();
        Box::pin(async move {
            let basis = match queue_client::get_string_arg("basis", &message)
                .ok_or("no basis specified for build!".to_string())
                .map(|b| core::parse_basis(b).map_err(|_| "failed to parse basis!".to_string()))
            {
                Ok(Ok(b)) => b,
                Err(e) | Ok(Err(e)) => return ConsumeResult::Failure(e, Vec::new()),
            };

            let outputs = match query(basis).await {
                ConsumeResult::Success(o) => o,
                other => return other,
            };

            let client = _self.get_queue_client();
            let mut blocked = Vec::new();
            for artifact in &outputs {
                if &artifact.name != "target" && &artifact.name != "dependency" {
                    continue;
                }

                let mut args = ArtifactsBuilder::new();
                args.add_string(
                    "basis",
                    queue_client::get_string_arg("basis", &message)
                        .unwrap()
                        .to_string(),
                );
                args.add_string("target", artifact.value_string.clone());
                match client
                    .enqueue(
                        "builds".to_string(),
                        Message {
                            name: format!("build and test {}", artifact.value_string),
                            arguments: args.build(),
                            ..Default::default()
                        },
                    )
                    .await
                {
                    Ok(id) => {
                        blocked.push(BlockingMessage {
                            id,
                            queue: "builds".to_string(),
                        });
                    }
                    Err(_) => {
                        return ConsumeResult::Failure(
                            "failed to enqueue build task!".to_string(),
                            outputs,
                        )
                    }
                };
            }

            if blocked.is_empty() {
                ConsumeResult::Success(outputs)
            } else {
                ConsumeResult::Blocked(outputs, blocked)
            }
        })
    }
}

async fn checkout(basis: service::Basis) -> Result<(), std::io::Error> {
    std::fs::create_dir_all("/tmp/ci/work").unwrap();
    std::fs::create_dir_all("/tmp/ci/src").unwrap();

    let d = src_lib::Src::new(std::path::PathBuf::from("/tmp/ci/src"))?;
    d.checkout(std::path::PathBuf::from("/tmp/ci/work"), basis.clone())
        .await?;

    let alias = d.find_unused_alias("ci");
    d.set_change_by_alias(
        &alias,
        &service::Space {
            directory: "/tmp/ci/work".to_string(),
            basis: basis,
            ..Default::default()
        },
    );

    Ok(())
}

// Find affected targets based on the provided snapshot
async fn find_targets(snapshot: service::Snapshot) -> Result<Vec<String>, String> {
    let mut direct_targets = HashSet::new();

    for file in &snapshot.files {
        let output = match std::process::Command::new("bazel")
            .arg("query")
            .arg(format!(
                "attr('srcs', '{}', //...)",
                path_to_bazel(&file.path)
            ))
            .current_dir("/tmp/ci/work")
            .output()
        {
            Ok(o) => o,
            Err(e) => return Err(format!("failed to run bazel query rdeps command: {e:?}")),
        };
        let stdout = std::str::from_utf8(&output.stdout).unwrap().trim();
        if !output.status.success() {
            return Err("failed to run bazel query command!".to_string());
        }
        for line in stdout.lines() {
            direct_targets.insert(line.trim().to_owned());
        }
    }

    Ok(direct_targets.into_iter().collect())
}

// Find rdeps given a target list
async fn find_rdeps(targets: Vec<String>) -> Result<Vec<String>, String> {
    let mut indirect_targets = HashSet::new();

    for target in &targets {
        let output = match std::process::Command::new("bazel")
            .arg("query")
            .arg(format!("rdeps(//..., {})", target))
            .current_dir("/tmp/ci/work")
            .output()
        {
            Ok(o) => o,
            Err(e) => return Err(format!("failed to run bazel query rdeps command: {e:?}")),
        };
        let stdout = std::str::from_utf8(&output.stdout).unwrap().trim();
        if !output.status.success() {
            return Err("failed to run bazel query rdeps command!".to_string());
        }
        for line in stdout.lines() {
            indirect_targets.insert(line.trim().to_owned());
        }
    }

    // Remove indirect targets
    for target in &targets {
        indirect_targets.remove(target);
    }

    Ok(indirect_targets.into_iter().collect())
}

async fn query(basis: service::Basis) -> ConsumeResult {
    let mut outputs = ArtifactsBuilder::new();

    if let Err(e) = checkout(basis.clone()).await {
        return ConsumeResult::Failure(format!("failed to checkout: {e:?}"), outputs.build());
    }

    let directs = match find_targets(service::Snapshot {
        timestamp: 123,
        basis: basis.clone(),
        files: vec![service::FileDiff {
            path: "util/pool/pool.rs".to_string(),
            ..Default::default()
        }],
        message: String::new(),
    })
    .await
    {
        Ok(d) => d,
        Err(e) => {
            return ConsumeResult::Failure(
                format!("failed to find targets: {e:?}"),
                outputs.build(),
            )
        }
    };

    let indirects = match find_rdeps(directs.clone()).await {
        Ok(d) => d,
        Err(e) => {
            return ConsumeResult::Failure(
                format!("failed to find dependencies: {e:?}"),
                outputs.build(),
            )
        }
    };

    for direct in directs {
        outputs.add_string("target", direct);
    }

    for indirect in indirects {
        outputs.add_string("dependency", indirect);
    }

    ConsumeResult::Success(outputs.build())
}

async fn build(req: Message) -> ConsumeResult {
    let mut outputs = ArtifactsBuilder::new();

    let basis = match queue_client::get_string_arg("basis", &req)
        .ok_or("no basis specified for build!".to_string())
        .map(|b| core::parse_basis(b).map_err(|_| "failed to parse basis!".to_string()))
    {
        Ok(Ok(b)) => b,
        Err(e) | Ok(Err(e)) => return ConsumeResult::Failure(e, outputs.build()),
    };

    if let Err(e) = checkout(basis.clone()).await {
        return ConsumeResult::Failure(format!("failed to checkout: {e:?}"), outputs.build());
    }

    let target = match queue_client::get_string_arg("target", &req) {
        Some(t) => t,
        None => {
            return ConsumeResult::Failure(
                "no target specified for build!".to_string(),
                outputs.build(),
            )
        }
    };
    let optimized = queue_client::get_bool_arg("optimized", &req).unwrap_or(false);

    // Run build
    let mut cmd = std::process::Command::new("bazel");
    cmd.arg("build");
    if optimized {
        cmd.arg("-c").arg("opt");
    }
    cmd.arg(&target);

    let output = match cmd.current_dir("/tmp/ci/work").output() {
        Ok(o) => o,
        Err(e) => {
            return ConsumeResult::Failure(format!("failed to run build: {e:?}"), outputs.build());
        }
    };

    let stdout = std::str::from_utf8(&output.stdout).unwrap().trim();
    let stderr = std::str::from_utf8(&output.stderr).unwrap().trim();

    outputs.add_string("build_output", format!("{stdout}\n{stderr}"));

    if output.status.success() {
        outputs.add_bool("build_success", true);
    } else {
        outputs.add_bool("build_success", false);
        return ConsumeResult::Failure("build failed".to_string(), outputs.build());
    }

    // Run tests
    let mut cmd = std::process::Command::new("bazel");
    cmd.arg("test");
    if optimized {
        cmd.arg("-c").arg("opt");
    }
    cmd.arg(&target);
    let output = match cmd.current_dir("/tmp/ci/work").output() {
        Ok(o) => o,
        Err(e) => {
            return ConsumeResult::Failure(format!("failed to run tests: {e:?}"), outputs.build());
        }
    };

    let stdout = std::str::from_utf8(&output.stdout).unwrap().trim();
    let stderr = std::str::from_utf8(&output.stderr).unwrap().trim();

    outputs.add_string("test_output", format!("{stdout}\n{stderr}"));

    if output.status.success() || output.status.code() == Some(4) {
        outputs.add_bool("test_success", true);
    } else {
        outputs.add_bool("test_success", false);
        return ConsumeResult::Failure("tests failed".to_string(), outputs.build());
    }

    ConsumeResult::Success(outputs.build())
}

fn path_to_bazel(path: &str) -> String {
    let (start, end) = match path.rfind('/') {
        Some(idx) => (&path[..idx], &path[idx + 1..]),
        None => ("", path),
    };
    format!("//{start}:{end}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bazel_target_conversion() {
        assert_eq!(path_to_bazel("asdf.txt"), "//:asdf.txt");
        assert_eq!(path_to_bazel("tools/ci/main.rs"), "//tools/ci:main.rs");
    }
}
