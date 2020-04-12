use auth_client::AuthServer;
use rand::Rng;
use std::io::{BufRead, Write};
use std::process::{Command, Stdio};

pub fn wait_for_enter() {
    std::io::stdout().flush();
    for line in std::io::stdin().lock().lines() {
        break;
    }
}

// If the authentication token exists, will read from that token. Doesn't check if it's valid. If
// not set, just gives back an empty string.
pub fn load_auth() -> String {
    let home = std::env::var("HOME").unwrap();
    let auth_path = format!("{}/.x20/auth_token", home);
    if let Ok(token) = std::fs::read_to_string(&auth_path) {
        return token;
    }

    String::new()
}

pub fn load_and_check_auth(auth: auth_client::AuthClient) -> String {
    let home = std::env::var("HOME").unwrap();
    let auth_path = format!("{}/.x20/auth_token", home);
    let token = match std::fs::read_to_string(&auth_path) {
        Ok(token) => {
            let response = auth.authenticate(token.clone());
            if response.get_success() {
                return token;
            }
        }
        Err(_) => (),
    };

    println!("You need to log in.");
    loop {
        println!("Getting challenge...");
        let mut challenge = auth.login();
        println!(
            "Visit this URL: \n\n{}\n\nthen press enter when you've logged in.",
            challenge.get_url()
        );

        wait_for_enter();

        println!("Challenge got");

        let response = auth.authenticate(challenge.get_token().to_string());
        if response.get_success() {
            std::fs::write(&auth_path, challenge.get_token());
            return challenge.take_token();
        }

        println!("That didn't work. Let's try again.");
    }
}

pub fn edit_string(input: &str) -> Result<String, ()> {
    let editor = match std::env::var("EDITOR") {
        Ok(x) => x,
        Err(_) => String::from("nano"),
    };
    let filename = format!("/tmp/{}", rand::thread_rng().gen::<u64>());
    std::fs::write(&filename, input).unwrap();

    let output = match Command::new(&editor)
        .arg(&filename)
        .stdout(Stdio::inherit())
        .stdin(Stdio::inherit())
        .output()
    {
        Ok(out) => out,
        Err(_) => {
            println!("unable to start editor: {}", editor);
            return Err(());
        }
    };

    if !output.status.success() {
        return Err(());
    }

    std::fs::read_to_string(&filename).map_err(|_| ())
}

pub struct Description {
    pub title: String,
    pub description: String,
    pub tags: Vec<(String, String)>,
}

impl Description {
    pub fn new() -> Self {
        Self {
            title: String::new(),
            description: String::new(),
            tags: Vec::new(),
        }
    }

    pub fn from_str(input: &str) -> Self {
        let mut output = Self::new();

        let mut description = Vec::new();
        for line in input.split("\n") {
            let trimmed = line.trim();

            // Ignore comment lines
            if trimmed.starts_with("#") {
                continue;
            }

            // Check if the line is a tag
            let split: Vec<_> = trimmed.split("=").collect();
            if split.len() == 2 {
                let mut is_valid_tag = true;
                if split[0].len() == 0 {
                    is_valid_tag = false;
                }
                for ch in split[0].chars() {
                    if !ch.is_uppercase() {
                        is_valid_tag = false;
                        break;
                    }
                }

                if is_valid_tag {
                    output
                        .tags
                        .push((split[0].to_string(), split[1].to_string()));
                    continue;
                }
            }

            // If the line is too full of spaces, strip them.
            if line.starts_with("   ") {
                description.push(trimmed);
                continue;
            }

            description.push(line);
        }

        // Remove starting and trailing newlines.
        let mut sliced_desc = description.as_slice();
        while let Some(&"") = sliced_desc.first() {
            sliced_desc = &sliced_desc[1..];
        }

        while let Some(&"") = sliced_desc.last() {
            sliced_desc = &sliced_desc[..sliced_desc.len() - 1];
        }

        output.description = sliced_desc.join("\n");

        let mut title = match output.description.lines().next() {
            Some(t) => t.to_owned(),
            None => String::new(),
        };
        title.truncate(80);
        output.title = title;

        if !output.title.is_empty() {
            let mut lines_iter = output.description.lines();
            lines_iter.next();
            output.description = lines_iter.collect::<Vec<_>>().join("\n");
        }

        output
    }

    pub fn to_string(&self) -> String {
        format!(
            "{}\n{}\n\n{}",
            self.title,
            self.description,
            self.tags
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("\n")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_description() {
        let text = "
            This is a change description
            R=person1,person2
            # Comment
            another line

            and another line

        ";
        let d = Description::from_str(text);
        assert_eq!(&d.title, "This is a change description");
        assert_eq!(
            d.tags,
            vec![(String::from("R"), String::from("person1,person2"))]
        );
        assert_eq!(&d.description, "another line\n\nand another line");

        assert_eq!(
            &d.to_string(),
            "This is a change description\nanother line\n\nand another line\n\nR=person1,person2"
        );
    }
}
