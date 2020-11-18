#[derive(Debug)]
pub enum GitError {
    NotConfigured,
    CommandFailed(String),
}

pub fn get_stdout(mut c: std::process::Command) -> Result<String, GitError> {
    match c.output() {
        Ok(result) => {
            if !result.status.success() {
                let output_stderr = std::str::from_utf8(&result.stderr)
                    .unwrap()
                    .trim()
                    .to_owned();
                return Err(GitError::CommandFailed(output_stderr));
            }

            let output_stdout = std::str::from_utf8(&result.stdout)
                .unwrap()
                .trim()
                .to_owned();
            Ok(output_stdout)
        }
        Err(e) => Err(GitError::CommandFailed(format!("{:?}", e))),
    }
}

pub fn check_branch_exists(branch: &str) -> Result<bool, GitError> {
    let mut c = std::process::Command::new("git");
    c.arg("rev-parse");
    c.arg("--verify");
    c.arg(branch);
    match get_stdout(c) {
        Ok(_) => Ok(true),
        Err(CommandFailed) => Ok(false),
        Err(x) => Err(x),
    }
}

pub fn checkout(branch: &str, create: bool) -> Result<(), GitError> {
    let mut c = std::process::Command::new("git");
    c.arg("checkout");
    if create {
        c.arg("-b");
    }
    c.arg(branch);
    get_stdout(c)?;
    Ok(())
}

pub fn pull() -> Result<(), GitError> {
    let mut c = std::process::Command::new("git");
    c.arg("pull");
    get_stdout(c)?;
    Ok(())
}

pub fn merge(branch: &str) -> Result<(), GitError> {
    let mut c = std::process::Command::new("git");
    c.arg("merge")
        .arg(branch)
        .stdout(std::process::Stdio::inherit())
        .stdin(std::process::Stdio::inherit());

    get_stdout(c)?;
    Ok(())
}

pub fn push() -> Result<(), GitError> {
    let mut c = std::process::Command::new("git");
    c.arg("push");
    c.arg("-u");
    c.arg("origin");
    get_stdout(c)?;
    Ok(())
}

pub fn create_pull_request() -> Result<String, GitError> {
    let mut c = std::process::Command::new("hub");
    c.arg("pull-request")
        .stdout(std::process::Stdio::inherit())
        .stdin(std::process::Stdio::inherit());
    get_stdout(c)
}

pub fn merge_base(branch1: &str, branch2: &str) -> Result<String, GitError> {
    let mut c = std::process::Command::new("git");
    c.arg("merge-base");
    c.arg(branch1);
    c.arg(branch2);
    get_stdout(c)
}

pub fn add_all() -> Result<(), GitError> {
    let mut c = std::process::Command::new("git");
    c.arg("add");
    c.arg(".");
    get_stdout(c)?;
    Ok(())
}

pub fn get_branch_name() -> Result<String, GitError> {
    let mut c = std::process::Command::new("git");
    c.arg("rev-parse");
    c.arg("--abbrev-ref");
    c.arg("HEAD");
    get_stdout(c)
}

pub fn check_for_pr() -> Option<String> {
    let mut c = std::process::Command::new("hub");
    c.arg("pr");
    c.arg("show");
    c.arg("-u");
    match get_stdout(c) {
        Ok(s) => Some(s),
        _ => None,
    }
}

pub fn diff(branch: &str, main_branch: &str) -> Result<String, GitError> {
    let base = merge_base(branch, main_branch)?;

    let mut c = std::process::Command::new("git");
    c.arg("diff");
    c.arg(base);
    c.arg(branch)
        .stdout(std::process::Stdio::inherit())
        .stdin(std::process::Stdio::inherit());

    get_stdout(c)
}

pub fn diff_file(branch: &str, main_branch: &str, file: &str) -> Result<String, GitError> {
    let base = merge_base(branch, main_branch)?;

    let mut c = std::process::Command::new("git");
    c.arg("diff");
    c.arg("--no-color");
    c.arg("--no-ext-diff");
    c.arg("-U0");
    c.arg(base);
    c.arg("--");
    c.arg(file)
        .stdout(std::process::Stdio::inherit())
        .stdin(std::process::Stdio::inherit());

    get_stdout(c)
}

pub fn files(current_branch: &str, main_branch: &str) -> Result<Vec<String>, GitError> {
    let base = merge_base(current_branch, main_branch)?;

    let mut c = std::process::Command::new("git");
    c.arg("--no-pager").arg("diff").arg(base).arg("--name-only");

    let out = get_stdout(c)?;
    let mut output: Vec<_> = out
        .split("\n")
        .map(|x| x.trim().to_string())
        .filter(|x| !x.is_empty())
        .collect();

    let mut c = std::process::Command::new("git");
    c.arg("ls-files").arg("--others").arg("--exclude-standard");

    let out = get_stdout(c)?;
    for result in out
        .split("\n")
        .map(|x| x.trim().to_string())
        .filter(|x| !x.is_empty())
    {
        output.push(result);
    }

    Ok(output)
}

pub fn commit() -> Result<(), GitError> {
    let branch_name = get_branch_name()?;

    let mut c = std::process::Command::new("git");
    c.arg("commit");
    c.arg("-n");
    c.arg("-m");
    c.arg(format!("fix:{}", branch_name));

    match get_stdout(c) {
        Ok(_) => Ok(()),
        Err(GitError::CommandFailed(msg)) => {
            if msg.contains("nothing to commit") {
                return Ok(());
            }
            return Err(GitError::CommandFailed(msg));
        }
        Err(e) => Err(e),
    }
}

pub fn get_root_directory() -> Result<String, GitError> {
    let mut c = std::process::Command::new("git");
    c.arg("rev-parse");
    c.arg("--show-toplevel");
    get_stdout(c)
}
