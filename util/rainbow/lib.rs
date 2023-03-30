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

pub fn resolve_binary(name: &str, tag: &str) -> Option<String> {
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

    let mut f = gfile::GFile::create(format!("/cns/rainbow-binaries/{name}/{sha}"))?;
    std::io::copy(&mut std::fs::File::open(path)?, &mut f)?;

    for tag in tags {
        let mut f = gfile::GFile::create(format!("/cns/rainbow-binaries/{name}/tags/{tag}"))?;
        f.write_all(
            format!("https://storage.googleapis.com/rainbow-binaries/{name}/{sha}").as_bytes(),
        )?;
    }

    Ok(sha)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_resolve() {
        let client = auth_client::AuthClient::new_tls("auth.colinmerkel.xyz", 8888);
        let token = cli::load_auth();
        client.global_init(token);
        assert_eq!(
            resolve_binary("ws_example", "test"),
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
