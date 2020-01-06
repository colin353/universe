use rand;
use recordio::{RecordIOReader, RecordIOWriter};
use x20_client;

use std::collections::HashMap;
use std::fs::File;
use std::os::unix::fs::PermissionsExt;

pub struct X20Manager {
    client: x20_client::X20Client,
    base_dir: String,
}

impl X20Manager {
    pub fn new(client: x20_client::X20Client, base_dir: String) -> Self {
        Self {
            client: client,
            base_dir: base_dir,
        }
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

    pub fn write_saved_binaries(&self, bins: &[x20::Binary]) {
        let mut f = File::create(&format!("{}/config/binaries.recordio", self.base_dir)).unwrap();
        let mut w = RecordIOWriter::new(f);
        for bin in bins {
            w.write(bin);
        }
    }

    pub fn read_saved_binaries(&self) -> HashMap<String, x20::Binary> {
        let mut f = match File::open(&format!("{}/config/binaries.recordio", self.base_dir)) {
            Ok(f) => f,
            Err(_) => return HashMap::new(),
        };
        let mut buf = std::io::BufReader::new(f);
        let reader = RecordIOReader::<x20::Binary, _>::new(buf);

        let mut output = HashMap::new();
        for bin in reader {
            output.insert(bin.get_name().to_owned(), bin);
        }
        output
    }

    pub fn update(&self) {
        let existing_binaries = self.read_saved_binaries();
        let mut updated_binaries = Vec::new();
        let mut had_failure = false;
        let mut had_success = false;
        for binary in self.client.get_binaries() {
            if let Some(b) = existing_binaries.get(binary.get_name()) {
                if b.get_version() == binary.get_version() {
                    // No need to update - we already have the latest version
                    updated_binaries.push(binary);
                    continue;
                }
            }

            // Download the binary from the provided URL
            let output = match std::process::Command::new("curl")
                .arg("-sSfL")
                .arg("--output")
                .arg(format!("{}/bin/{}", self.base_dir, binary.get_name()))
                .arg(binary.get_url())
                .output()
            {
                Ok(o) => o,
                Err(e) => {
                    had_failure = true;
                    println!("❌failed to start download!\n\n {:?}", e);
                    break;
                }
            };

            let output_stderr = std::str::from_utf8(&output.stderr)
                .unwrap()
                .trim()
                .to_owned();
            if !output.status.success() {
                eprintln!(
                    "❌failed to download binary `{}`:\n\n {}",
                    binary.get_name(),
                    output_stderr
                );
                had_failure = true;
                break;
            }

            // Mark the file as exeuctable
            let f = match File::open(&format!("{}/bin/{}", self.base_dir, binary.get_name())) {
                Ok(f) => f,
                Err(_) => {
                    eprintln!(
                        "❌failed to set permissions for binary `{}`",
                        binary.get_name()
                    );
                    had_failure = true;
                    break;
                }
            };
            f.metadata().unwrap().permissions().set_mode(0o775);

            println!("✔️ Updated {}", binary.get_name());
            updated_binaries.push(binary);
            had_success = true;
        }

        if had_success {
            self.write_saved_binaries(&updated_binaries);
            println!("✔️ Saved metadata for {} binaries", updated_binaries.len());
        }

        if had_failure {
            std::process::exit(1);
        }

        println!("✔️ Everything is up to date");
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
