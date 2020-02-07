use config;
use rand;
use recordio::{RecordIOReader, RecordIOWriter};
use subprocess;
use x20_client;

use std::collections::HashMap;
use std::fs::File;

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
        println!("binaries: ");
        if binaries.len() == 0 {
            eprintln!("There are no binaries");
        }
        for bin in binaries {
            println!(
                " - {} (v{}{}) @ {}",
                bin.get_name(),
                bin.get_version(),
                if bin.get_target().is_empty() {
                    String::new()
                } else {
                    format!(" from {}", bin.get_target())
                },
                if !bin.get_docker_img().is_empty() {
                    format!("{}@{}", bin.get_docker_img(), bin.get_docker_img_tag())
                } else {
                    bin.get_url().to_string()
                }
            );
        }

        let env = self.read_saved_environment();
        let configs = self.client.get_configs(env);
        println!("\nconfigs: ");
        for config in configs {
            println!(" - {} (v{})", config.get_name(), config.get_version(),);
        }
    }

    pub fn read_saved_environment(&self) -> String {
        match std::fs::read_to_string(&format!("{}/config/env", self.base_dir)) {
            Ok(s) => s,
            Err(_) => String::new(),
        }
    }

    pub fn write_saved_environment(&self, env: String) {
        std::fs::write(&format!("{}/config/env", self.base_dir), env).unwrap()
    }

    pub fn write_saved_configs(&self, configs: &[x20::Configuration]) {
        let f = File::create(&format!("{}/config/configs.recordio", self.base_dir)).unwrap();
        let mut w = RecordIOWriter::new(f);
        for bin in configs {
            w.write(bin);
        }
    }

    pub fn read_saved_configs(&self) -> Vec<x20::Configuration> {
        let f = match File::open(&format!("{}/config/configs.recordio", self.base_dir)) {
            Ok(f) => f,
            Err(_) => return Vec::new(),
        };
        let buf = std::io::BufReader::new(f);
        let reader = RecordIOReader::<x20::Configuration, _>::new(buf);

        let mut output = Vec::new();
        for cfg in reader {
            output.push(cfg);
        }
        output
    }

    pub fn write_saved_binaries(&self, bins: &[x20::Binary]) {
        let f = File::create(&format!("{}/config/binaries.recordio", self.base_dir)).unwrap();
        let mut w = RecordIOWriter::new(f);
        for bin in bins {
            w.write(bin);
        }
    }

    pub fn read_saved_binaries(&self) -> HashMap<String, x20::Binary> {
        let f = match File::open(&format!("{}/config/binaries.recordio", self.base_dir)) {
            Ok(f) => f,
            Err(_) => return HashMap::new(),
        };
        let buf = std::io::BufReader::new(f);
        let reader = RecordIOReader::<x20::Binary, _>::new(buf);

        let mut output = HashMap::new();
        for bin in reader {
            output.insert(bin.get_name().to_owned(), bin);
        }
        output
    }

    pub fn update(&self) -> (Vec<x20::Binary>, Vec<x20::Configuration>) {
        let existing_binaries = self.read_saved_binaries();
        let mut new_binaries = Vec::new();
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

            new_binaries.push(binary.clone());

            // If this is a docker image, there's no need to download anything.
            if !binary.get_docker_img().is_empty() {
                println!("✔️ Updated {}", binary.get_name());
                updated_binaries.push(binary);
                had_success = true;
                continue;
            }

            let temporary_location = format!("/tmp/x20_{}", binary.get_name());
            let location = format!("{}/bin/{}", self.base_dir, binary.get_name());

            // Download the binary from the provided URL
            let output = match std::process::Command::new("curl")
                .arg("-sSfL")
                .arg("--output")
                .arg(&temporary_location)
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

            let output = match std::process::Command::new("chmod")
                .arg("+x")
                .arg(&temporary_location)
                .output()
            {
                Ok(o) => o,
                Err(e) => {
                    println!("failed to chmod downloaded binary: {:?}", e);
                    had_failure = true;
                    break;
                }
            };

            let output_stderr = std::str::from_utf8(&output.stderr)
                .unwrap()
                .trim()
                .to_owned();
            if !output.status.success() {
                eprintln!("Failed to chmod binary:\n\n {}", output_stderr);
                had_failure = true;
                break;
            }

            // In case we are updating a running binary, we need to delete our bin file
            // and move the temporary binary overtop, or else we'll get some kind of error
            std::fs::remove_file(&location); // No need to unwrap - if it doesn't exist it's ok
            std::fs::copy(&temporary_location, &location).unwrap();
            std::fs::remove_file(&temporary_location).unwrap();

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

        let cfgs: HashMap<String, x20::Configuration> = self
            .read_saved_configs()
            .into_iter()
            .map(|cfg| (cfg.get_name().to_string(), cfg))
            .collect();

        let env = self.read_saved_environment();
        let new_cfgs = self.client.get_configs(env);
        let mut new_configs = Vec::new();

        for config in &new_cfgs {
            if let Some(c) = cfgs.get(config.get_name()) {
                if c.get_version() == config.get_version() {
                    continue;
                }
            }
            new_configs.push(config.clone());
        }

        self.write_saved_configs(&new_cfgs);
        if new_configs.len() > 0 {
            println!("✔️ Updated {} configs", new_configs.len());
        }
        println!("✔️ Everything is up to date");

        (new_binaries, new_configs)
    }

    pub fn publish(
        &self,
        name: String,
        path: String,
        target: String,
        source: String,
        docker_img: String,
        docker_img_tag: String,
        create: bool,
    ) {
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
            Some(mut binary) => {
                if !target.is_empty() {
                    binary.set_target(target);
                }

                if !source.is_empty() {
                    binary.set_source(source);
                }

                binary
            }
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
                b.set_source(source);
                b
            }
        };

        // Come up with a random name for the binary
        let name = format!("{:x}{:x}", rand::random::<u64>(), rand::random::<u64>());

        if !docker_img.is_empty() {
            binary.set_docker_img(docker_img);
        }
        if !docker_img_tag.is_empty() {
            binary.set_docker_img_tag(docker_img_tag);
        }

        if !binary.get_docker_img().is_empty() {
            if binary.get_docker_img_tag().is_empty() {
                eprintln!("❌you must provide --docker_img_tag for docker images!");
                std::process::exit(1);
            }
        } else {
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
        }

        let mut req = x20::PublishBinaryRequest::new();
        req.set_binary(binary);
        self.client.publish_binary(req);

        println!("✔️ published");
    }

    pub fn setconfig(&self, input_configuration: String) {
        let config = match config::generate_config(&input_configuration) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("❌invalid config: {:?}", e);
                std::process::exit(1);
            }
        };

        let mut req = x20::PublishConfigRequest::new();
        req.set_config(config);
        self.client.publish_config(req);

        println!("✔️ published");
    }

    // Start up all the configs associated with the configs
    pub fn start(&self) {
        let env = self.read_saved_environment();
        let mut configs = self.read_saved_configs();
        let binaries = self.read_saved_binaries();

        if configs.len() == 0 {
            eprintln!("❌You have no configs, so nothing will be started");
            eprintln!("❌FYI, your current environment is set to `{}`", env);
            eprintln!("❌If you'd like to change your environment, run:");
            eprintln!("  x20 env --env=desktop");
            std::process::exit(1);
        }

        configs.sort_by(|a, b| a.get_priority().cmp(&b.get_priority()));
        let mut children = Vec::new();
        let mut failed = false;
        for config in configs {
            let binary = match binaries.get(config.get_binary_name()) {
                Some(b) => b,
                None => {
                    eprintln!(
                        "❌Config referenced unknown binary `{}`!",
                        config.get_binary_name()
                    );
                    failed = true;
                    break;
                }
            };
            let mut child =
                subprocess::ChildProcess::new(self.base_dir.clone(), config, binary.to_owned());
            if child.config.get_long_running() {
                child.start();
                children.push(child);
                std::thread::sleep(std::time::Duration::from_millis(500));
            } else {
                let success = child.run_to_completion();
                if !success {
                    failed = true;
                    break;
                }
            }
        }

        let mut last_update_check = 0;
        let mut updated_binaries = HashMap::new();
        let mut updated_configs = HashMap::new();
        loop {
            if last_update_check > 60 {
                last_update_check = 0;
                let (bins, cfgs) = self.update();
                updated_binaries = bins
                    .into_iter()
                    .map(|b| (b.get_name().to_string(), b))
                    .collect();
                updated_configs = cfgs
                    .into_iter()
                    .map(|c| (c.get_name().to_string(), c))
                    .collect();
            }

            for child in &mut children {
                child.tail_logs();
                if !child.check_alive() {
                    failed = true;
                    break;
                }

                // Check if there are any updates that apply to this child
                let mut should_reload = false;
                if let Some(c) = updated_configs.get(child.config.get_name()) {
                    child.config = c.clone();
                    should_reload = true;
                }

                if let Some(b) = updated_binaries.get(child.binary.get_name()) {
                    child.binary = b.clone();
                    should_reload = true;
                }

                if should_reload {
                    if !child.reload() {
                        failed = true
                    }
                }
            }

            if failed {
                for child in &mut children {
                    child.kill();
                }

                std::process::exit(1);
            }

            updated_configs.clear();
            updated_binaries.clear();
            std::thread::sleep(std::time::Duration::from_secs(1));
            last_update_check += 1;
        }
    }
}
