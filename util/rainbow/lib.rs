use hyper::body::HttpBody;
use std::fmt::Write as _;
use std::io::{Read, Write};

pub fn is_valid_name(name: &str) -> bool {
    !name.contains("/")
}

pub fn parse(name: &str) -> std::io::Result<(&str, &str)> {
    let components: Vec<_> = name.split(":").collect();
    if components.len() != 2 {
        return Err(std::io::Error::from(std::io::ErrorKind::InvalidData));
    }

    Ok((components[0], components[1]))
}

pub async fn async_resolve(name: &str, tag: &str) -> Option<String> {
    let uri: hyper::Uri =
        format!("https://storage.googleapis.com/rainbow-binaries/{name}/tags/{tag}")
            .parse()
            .unwrap();

    let https = hyper_tls::HttpsConnector::new();
    let client = hyper::Client::builder().build::<_, hyper::Body>(https);
    let mut res = client.get(uri).await.ok()?;
    if !res.status().is_success() {
        return None;
    }

    let mut buf = Vec::new();
    while let Some(next) = res.data().await {
        let chunk = next.ok()?;
        buf.write_all(&chunk).ok()?;
    }

    Some(String::from_utf8(buf).unwrap())
}

pub fn resolve(name: &str, tag: &str) -> Option<String> {
    let path = format!("/cns/rainbow-binaries/{name}/tags/{tag}");
    let mut f = match gfile::GFile::open(&path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("error: {e:#?}");
            return None;
        }
    };
    let mut url = String::new();
    match f.read_to_string(&mut url) {
        Ok(_) => (),
        Err(e) => {
            eprintln!("read to url error: {e:#?}");
            return None;
        }
    };
    Some(url.trim().to_string())
}

pub fn publish(name: &str, tags: &[&str], path: &std::path::Path) -> std::io::Result<String> {
    let mut suffix = "";
    if let Some("tar") = path.extension().map(|s| s.to_str()).flatten() {
        suffix = ".tar";
    }

    let mut h = sha256::Sha256::new();
    let mut f = std::fs::File::open(path)?;
    let mut buf = vec![0_u8; 8192];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.absorb(&buf[0..n]);
    }
    let sha_bytes = h.finish();
    let mut sha = String::new();
    for byte in sha_bytes {
        write!(&mut sha, "{:02x}", byte).unwrap();
    }

    let mut f = gfile::GFile::create(format!("/cns/rainbow-binaries/{name}/{sha}{suffix}"))?;
    std::io::copy(&mut std::fs::File::open(path)?, &mut f)?;
    f.flush()?;

    for tag in tags {
        update_tag(
            name,
            tag,
            format!("https://storage.googleapis.com/rainbow-binaries/{name}/{sha}{suffix}"),
        )?;
    }

    Ok(sha)
}

pub fn update_tag(name: &str, tag: &str, new_target: String) -> std::io::Result<()> {
    // Create the tag file
    let tag_path = format!("/cns/rainbow-binaries/{name}/tags/{tag}");
    let mut f = gfile::GFile::create(&tag_path)?;
    f.write_all(new_target.as_bytes())?;

    // Read the tag log file, if it exists
    let log_path = format!("{tag_path}.log");
    let mut tag_log = match gfile::GFile::open(&log_path) {
        Ok(mut t) => {
            let mut buf = Vec::new();
            t.read_to_end(&mut buf)?;
            TagLog::from_bytes(&buf)?
        }
        Err(_) => TagLog::new(),
    };

    tag_log.push(time::timestamp(), new_target);
    tag_log.normalize();

    let mut f = gfile::GFile::create(&log_path)?;
    tag_log.write(&mut f)?;
    Ok(())
}

pub struct TagLog {
    pub entries: Vec<(u64, String)>,
}

impl TagLog {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn for_tag(name: &str, tag: &str) -> std::io::Result<Self> {
        let log_path = format!("/cns/rainbow-binaries/{name}/tags/{tag}.log");
        Ok(match gfile::GFile::open(&log_path) {
            Ok(mut t) => {
                let mut buf = Vec::new();
                t.read_to_end(&mut buf)?;
                TagLog::from_bytes(&buf)?
            }
            Err(_) => TagLog::new(),
        })
    }

    pub fn from_bytes(data: &[u8]) -> std::io::Result<Self> {
        let mut entries = Vec::new();
        let data = std::str::from_utf8(data).map_err(|_| {
            return std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "failed to parse tag log as utf8",
            );
        })?;
        for line in data.split('\n') {
            let components: Vec<_> = line.split(' ').collect();
            if components.len() != 2 {
                continue;
            }

            let time: u64 = match components[0].parse() {
                Ok(t) => t,
                Err(_) => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "failed to parse tag log timestamp",
                    ));
                }
            };

            entries.push((time, components[1].to_string()));
        }

        Ok(Self { entries })
    }

    pub fn normalize(&mut self) {
        self.entries.sort_by_key(|(ts, _)| *ts);
        self.entries.dedup_by(|a, b| a.1 == b.1);
        self.entries.truncate(20);
    }

    pub fn push(&mut self, ts: u64, dest: String) {
        self.entries.push((ts, dest));
    }

    pub fn write(&self, w: &mut dyn std::io::Write) -> std::io::Result<()> {
        for (ts, target) in &self.entries {
            write!(w, "{ts} {target}\n")?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_resolve() {
        let client = auth_client::AuthClient::new_tls("auth.colinmerkel.xyz", 8888);
        let token = cli::load_auth();
        client.global_init(token);
        assert_eq!(
            resolve("ws_example", "test"),
            Some("https://google.com".to_string())
        )
    }

    fn test_publish() {
        let client = auth_client::AuthClient::new_tls("auth.colinmerkel.xyz", 8888);
        let token = cli::load_auth();
        client.global_init(token);
        publish(
            "ws_example",
            &["test"],
            std::path::Path::new("/home/colin/bin/ws_example"),
        )
        .unwrap();
    }
}
