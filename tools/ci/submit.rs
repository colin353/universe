use lockserv_client::LockservClient;
use queue_client::{
    ArtifactsBuilder, BlockingMessage, ConsumeResult, Message, QueueClient, Status,
};
use std::future::Future;
use std::pin::Pin;

#[derive(Clone)]
pub struct SubmitConsumer {
    queue_client: QueueClient,
    lockserv_client: LockservClient,
}

impl SubmitConsumer {
    pub fn new(queue_client: QueueClient, lockserv_client: LockservClient) -> Self {
        Self {
            queue_client,
            lockserv_client,
        }
    }
}

impl queue_client::Consumer for SubmitConsumer {
    fn get_queue_client(&self) -> &QueueClient {
        &self.queue_client
    }

    fn get_lockserv_client(&self) -> &LockservClient {
        &self.lockserv_client
    }

    fn consume(&self, message: &Message) -> Pin<Box<dyn Future<Output = ConsumeResult> + Send>> {
        let message = message.clone();
        let _self = self.clone();
        Box::pin(async move {
            // Schedule the presubmit task first
            let basis = match queue_client::get_string_arg("basis", &message)
                .ok_or("no basis specified for submit!".to_string())
                .map(|b| core::parse_basis(b).map_err(|_| "failed to parse basis!".to_string()))
            {
                Ok(Ok(b)) => b,
                Err(e) | Ok(Err(e)) => return ConsumeResult::Failure(e, Vec::new()),
            };

            if basis.change == 0 {
                return ConsumeResult::Failure(
                    "a change must be specified to submit".to_string(),
                    Vec::new(),
                );
            }

            let client = _self.get_queue_client();
            let mut blocked = Vec::new();

            let mut args = ArtifactsBuilder::new();
            args.add_string(
                "basis",
                queue_client::get_string_arg("basis", &message)
                    .unwrap()
                    .to_string(),
            );

            match client
                .enqueue(
                    "presubmit".to_string(),
                    Message {
                        name: format!("presubmit {}", core::fmt_basis(basis.as_view())),
                        arguments: args.build(),
                        ..Default::default()
                    },
                )
                .await
            {
                Ok(id) => {
                    blocked.push(BlockingMessage {
                        id,
                        queue: "presubmit".to_string(),
                    });
                }
                Err(_) => {
                    return ConsumeResult::Failure(
                        "failed to enqueue presubmit task!".to_string(),
                        Vec::new(),
                    )
                }
            }

            ConsumeResult::Blocked(Vec::new(), blocked)
        })
    }

    fn resume(&self, message: &Message) -> Pin<Box<dyn Future<Output = ConsumeResult> + Send>> {
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

            let git_repository = queue_client::get_string_arg("git_repository", &message);

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
                    Err(_) => {
                        return ConsumeResult::Failure(
                            "failed to read blocking task due to RPC error!".to_string(),
                            Vec::new(),
                        )
                    }
                };

                if m.status != Status::Success {
                    return ConsumeResult::Failure("presubmit failed".to_string(), Vec::new());
                }
            }

            if let Some(destination) = git_repository {
                if let Err(e) = sync_to_github(basis.clone(), destination.to_string()).await {
                    return ConsumeResult::Failure(
                        format!("failed to sync to github: {e:?}"),
                        Vec::new(),
                    );
                }
            }

            if let Err(e) = submit(basis.clone()).await {
                return ConsumeResult::Failure(format!("failed to submit: {e:?}"), Vec::new());
            }

            ConsumeResult::Success(Vec::new())
        })
    }
}

async fn sync_to_github(basis: service::Basis, destination: String) -> Result<(), std::io::Error> {
    // Check out the code to sync to github
    crate::checkout(basis.clone()).await?;

    // Clone the target repository
    std::fs::remove_dir_all("/tmp/ci/github").unwrap();
    std::fs::create_dir_all("/tmp/ci/github").unwrap();
    let output = std::process::Command::new("git")
        .env("GIT_TERMINAL_PROMPT", "0")
        .arg("clone")
        .arg(&destination)
        .arg(".")
        .current_dir("/tmp/ci/github")
        .output()?;
    if !output.status.success() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!(
                "git clone failed: {}{}",
                std::str::from_utf8(&output.stdout).unwrap_or(""),
                std::str::from_utf8(&output.stderr).unwrap_or(""),
            ),
        ));
    }

    // Remove everything in the git repo
    let output = std::process::Command::new("rm")
        .arg("-rf")
        .arg("/tmp/ci/github/*")
        .output()?;
    if !output.status.success() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!(
                "rm -rf failed: {}{}",
                std::str::from_utf8(&output.stdout).unwrap_or(""),
                std::str::from_utf8(&output.stderr).unwrap_or(""),
            ),
        ));
    }

    // Copy everything into the git repo
    for entry in std::fs::read_dir("/tmp/ci/work")? {
        let entry = entry?;
        let ft = entry.file_type()?;
        if ft.is_dir() {
            copy(
                entry.path(),
                std::path::PathBuf::from("/tmp/ci/github").join(entry.file_name()),
            )?;
        } else if ft.is_file() {
            std::fs::copy(
                entry.path(),
                std::path::PathBuf::from("/tmp/ci/github").join(entry.file_name()),
            )
            .map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!(
                        "failed to copy (outer) from {:?} --> {:?}",
                        entry.path(),
                        std::path::PathBuf::from("/tmp/ci/github").join(entry.file_name()),
                    ),
                )
            })?;
        }
    }

    // Commit and push
    let output = std::process::Command::new("git")
        .env("GIT_TERMINAL_PROMPT", "0")
        .arg("add")
        .arg(".")
        .current_dir("/tmp/ci/github")
        .output()?;
    if !output.status.success() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!(
                "git add failed: {}{}",
                std::str::from_utf8(&output.stdout).unwrap_or(""),
                std::str::from_utf8(&output.stderr).unwrap_or(""),
            ),
        ));
    }

    let output = std::process::Command::new("git")
        .env("GIT_TERMINAL_PROMPT", "0")
        .arg("commit")
        .arg("-m")
        .arg(core::fmt_basis(basis.as_view()))
        .current_dir("/tmp/ci/github")
        .output()?;
    if !output.status.success() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!(
                "git commit failed: {}{}",
                std::str::from_utf8(&output.stdout).unwrap_or(""),
                std::str::from_utf8(&output.stderr).unwrap_or(""),
            ),
        ));
    }

    /*
    std::process::Command::new("git")
        .env("GIT_TERMINAL_PROMPT", "0")
        .arg("push")
        .current_dir("/tmp/ci/github")
        .output()?;
    */

    Ok(())
}

fn copy(
    src: impl AsRef<std::path::Path>,
    dest: impl AsRef<std::path::Path>,
) -> std::io::Result<()> {
    std::fs::create_dir_all(dest.as_ref());
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ft = entry.file_type()?;
        if ft.is_dir() {
            copy(entry.path(), dest.as_ref().join(entry.file_name()))?;
        } else if ft.is_file() {
            std::fs::copy(entry.path(), dest.as_ref().join(entry.file_name())).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!(
                        "failed to copy from {:?} --> {:?}",
                        entry.path(),
                        dest.as_ref().join(entry.file_name())
                    ),
                )
            })?;
        }
    }
    Ok(())
}

async fn submit(basis: service::Basis) -> Result<(), std::io::Error> {
    let d = src_lib::Src::new(std::path::PathBuf::from("/tmp/ci/src"))?;

    // Submit the change
    let client = d.get_client(&basis.host)?;
    let token = match d.get_identity(&basis.host) {
        Some(t) => t,
        None => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "no identity to submit with",
            ))
        }
    };
    let resp = client
        .submit(service::SubmitRequest {
            token,
            repo_owner: basis.owner.clone(),
            repo_name: basis.name.clone(),
            change_id: basis.change,
            snapshot_timestamp: basis.index,
        })
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, format!("{e:?}")))?;

    if resp.failed {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("failed to submit: {}", resp.error_message),
        ));
    }

    // TODO: check that it didn't fail

    Ok(())
}
