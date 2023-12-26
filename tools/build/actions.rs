use futures::StreamExt;
use std::io::Write;
use tokio::runtime::Runtime;

use crate::core::{BuildActions, BuildResult, Context};

async fn download(mut url: hyper::Uri, dest: std::path::PathBuf) -> std::io::Result<()> {
    let https = hyper_tls::HttpsConnector::new();
    let client: hyper::Client<hyper_tls::HttpsConnector<_>> = hyper::Client::builder().build(https);
    for _ in 0..3 {
        let res = client
            .get(url)
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::NotConnected, e))?;

        if res.status() == hyper::StatusCode::FOUND {
            let loc = res
                .headers()
                .get(hyper::header::LOCATION)
                .expect("302 without location");

            url = loc
                .to_str()
                .expect("failed to parse loc header")
                .parse()
                .expect("failed to parse redirect URL");

            continue;
        }

        if !res.status().is_success() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                format!("invalid status code: {}", res.status()),
            ));
        }

        let mut f = std::fs::File::create(&dest)?;
        let mut body = res.into_body();
        while let Some(chunk) = body.next().await {
            let chunk =
                chunk.map_err(|e| std::io::Error::new(std::io::ErrorKind::BrokenPipe, e))?;
            f.write_all(&chunk)?;
        }
        return Ok(());
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "too many redirects!",
    ))
}

impl BuildActions {
    pub fn new() -> Self {
        Self {}
    }

    pub fn run_process<P: Into<std::path::PathBuf>, S>(
        &self,
        context: &Context,
        bin: P,
        args: &[S],
    ) -> std::io::Result<Vec<u8>>
    where
        S: AsRef<str>,
    {
        let bin = bin.into();
        let mut cmd_debug = format!("{bin:?}");
        let mut c = std::process::Command::new(bin);
        for arg in args {
            cmd_debug.push(' ');
            cmd_debug.push_str(arg.as_ref());
            c.arg(arg.as_ref());
        }
        println!("run_command: {cmd_debug}");

        let out = c.output()?;
        if !out.status.success() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "command failed: {}",
                    std::str::from_utf8(&out.stderr).unwrap_or("")
                ),
            ));
        }

        Ok(out.stdout)
    }

    pub fn download<S: Into<String>, P: Into<std::path::PathBuf>>(
        &self,
        context: &Context,
        url: S,
        dest: P,
    ) -> std::io::Result<()> {
        let rt = Runtime::new().unwrap();
        let handle = rt.handle();

        let dest = dest.into();
        if let Some(p) = dest.parent() {
            std::fs::create_dir_all(p).ok();
        }
        let url = url.into();
        println!("download URL: {url}");
        let url = url.parse().map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("invalid URL: {}", url),
            )
        })?;

        handle.block_on(download(url, dest))
    }
}
