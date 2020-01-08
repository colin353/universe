use std::io::BufRead;
use std::io::Seek;

pub struct ChildProcess {
    config: x20::Configuration,
    offset: u64,
    log_file: String,
    binary_file: String,
    child: Option<std::process::Child>,
}

impl ChildProcess {
    pub fn new(base_dir: String, config: x20::Configuration) -> Self {
        let log_file = format!("{}/logs/{}", base_dir, config.get_binary_name());
        let binary_file = format!("{}/bin/{}", base_dir, config.get_binary_name());

        ChildProcess {
            config: config,
            offset: 0,
            log_file: log_file,
            binary_file: binary_file,
            child: None,
        }
    }

    pub fn start(&mut self) {
        let mut f = std::fs::File::create(&self.log_file).unwrap();
        let mut f2 = std::fs::File::create(&self.log_file).unwrap();
        println!("start bin file: {}", self.binary_file);
        let mut c = std::process::Command::new(&self.binary_file);
        c.stdout(f);
        c.stderr(f2);
        for arg in self.config.get_arguments() {
            c.arg(format!("{}={}", arg.get_name(), arg.get_value()));
        }
        self.child = Some(c.spawn().unwrap());
        println!("✔️ started `{}`", self.config.get_name());
    }

    pub fn tail_logs(&mut self) {
        let mut f = std::fs::File::open(&self.log_file).unwrap();
        f.seek(std::io::SeekFrom::Start(self.offset)).unwrap();
        let mut reader = std::io::BufReader::new(f);
        let mut buf = String::new();
        while let Ok(length) = reader.read_line(&mut buf) {
            if length == 0 {
                break;
            }
            println!("[{}] {}", self.config.get_name(), buf.trim());
            buf.clear();
            self.offset += length as u64;
        }
    }

    pub fn check_alive(&mut self) {
        if let Some(child) = self.child.as_mut() {
            if let Some(exit_status) = child.try_wait().unwrap() {
                println!(
                    "❌process `{}` terminated with exit status {:?}",
                    self.config.get_name(),
                    exit_status
                );
                println!("❌shutting everything down",);
                std::process::exit(1);
            }
        }
    }
}
