use rand;
use x20_client;

pub struct X20Manager {
    client: x20_client::X20Client,
}

impl X20Manager {
    pub fn new(client: x20_client::X20Client) -> Self {
        Self { client: client }
    }

    pub fn list(&self) {
        let binaries = self.client.get_binaries();
        if binaries.len() == 0 {
            eprintln!("There are no binaries");
        }
        for bin in binaries {
            println!(
                "{} (v{}) @ {}",
                bin.get_name(),
                bin.get_version(),
                bin.get_url()
            );
        }
    }

    pub fn publish(&self, name: String, path: String, target: String, create: bool) {
        if name.is_empty() && target.is_empty() {
            eprintln!("You must specify either a name (--name) or a target (--target) to publish");
            std::process::exit(1);
        }

        // Look up the details of the binary we are adding
        let mut binary = match self
            .client
            .get_binaries()
            .into_iter()
            .find(|b| b.get_name() == name)
        {
            Some(b) => b,
            None => {
                if !create {
                    eprintln!(
                        "A binary named `{}` doesn't exist. To create it, use --create=true",
                        name
                    );
                    std::process::exit(1);
                }

                let mut b = x20::Binary::new();
                b.set_name(name);
                b.set_target(target);
                b
            }
        };

        // Come up with a random name for the binary
        let name = format!("{:x}{:x}", rand::random::<u64>(), rand::random::<u64>());

        // Upload the binary to the cloud bucket
        let output = match std::process::Command::new("gsutil")
            .arg("cp")
            .arg(path)
            .arg(format!("gs://x20-binaries/{}", name))
            .output()
        {
            Ok(o) => o,
            Err(e) => {
                println!(
                    "failed to start gsutil copy. do you have gsutil installed? {:?}",
                    e
                );
                return;
            }
        };

        let output_stderr = std::str::from_utf8(&output.stderr)
            .unwrap()
            .trim()
            .to_owned();
        if !output.status.success() {
            eprintln!("Failed to upload binary:\n\n {}", output_stderr);
            return;
        }

        // Set the downloadable URL
        binary.set_url(format!(
            "https://storage.googleapis.com/x20-binaries/{}",
            name
        ));

        let mut req = x20::PublishBinaryRequest::new();
        req.set_binary(binary);
        self.client.publish_binary(req);

        println!("published");
    }
}
