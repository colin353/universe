use std::io::Read;

pub fn is_valid_name(name: &str) -> bool {
    !name.contains("/")
}

pub fn resolve_binary(name: &str, tag: &str) -> Option<String> {
    let path = format!("/cns/rainbow-binaries/{name}/tags/{tag}");
    let mut f = match gfile::GFile::open(&path) {
        Ok(f) => f,
        Err(e) => {
            return None;
        }
    };
    let mut url = String::new();
    f.read_to_string(&mut url).ok()?;
    Some(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve() {
        let client = auth_client::AuthClient::new_tls("auth.colinmerkel.xyz", 8888);
        let token = cli::load_auth();
        client.global_init(token);
        assert_eq!(
            resolve_binary("ws_example", "test"),
            Some("https://google.com".to_string())
        )
    }
}
