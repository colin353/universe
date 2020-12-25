#[macro_use]
extern crate flags;

use std::path::{Path, PathBuf};

#[derive(Debug)]
struct TTLDirectory {
    path: PathBuf,
    ttl_seconds: u64,
}

impl TTLDirectory {
    fn from_dir(entry: &std::fs::DirEntry) -> Result<Self, ()> {
        let filename = entry.file_name().into_string().unwrap();
        if !filename.starts_with("ttl=") {
            return Err(());
        }

        let ttl_spec = &filename[4..];
        if ttl_spec.len() < 2 {
            return Err(());
        }

        let ttl: u64 = match &ttl_spec[0..ttl_spec.len() - 1].parse() {
            Ok(x) => *x,
            Err(_) => {
                return Err(());
            }
        };

        let ttl_seconds = match &ttl_spec[ttl_spec.len() - 1..] {
            "s" => ttl,
            "m" => 60 * ttl,
            "h" => 3600 * ttl,
            "d" => 24 * 3600 * ttl,
            _ => {
                return Err(());
            }
        };

        Ok(Self {
            path: entry.path(),
            ttl_seconds,
        })
    }
}

fn traverse_dir(dir: &Path, output: &mut Vec<TTLDirectory>) {
    let results = match std::fs::read_dir(dir) {
        Ok(x) => x,
        Err(_) => return,
    };

    for result in results {
        let result = result.unwrap();

        if !result.file_type().unwrap().is_dir() {
            continue;
        }

        match TTLDirectory::from_dir(&result) {
            Ok(d) => {
                output.push(d);
                continue;
            }
            Err(_) => (),
        };

        traverse_dir(&result.path(), output);
    }
}

fn clean_dir(dir: &Path, ttl_seconds: u64) {
    let results = match std::fs::read_dir(dir) {
        Ok(x) => x,
        Err(_) => return,
    };

    for result in results {
        let result = result.unwrap();

        if result.file_type().unwrap().is_dir() {
            clean_dir(&result.path(), ttl_seconds);
            continue;
        }

        if result
            .metadata()
            .unwrap()
            .modified()
            .unwrap()
            .elapsed()
            .unwrap()
            > std::time::Duration::from_secs(ttl_seconds)
        {
            println!("cleaning up {:?}", result.path());
            std::fs::remove_file(result.path());
        }
    }
}

fn main() {
    let args = parse_flags!();

    let mut output = Vec::new();
    for arg in args {
        traverse_dir(Path::new(&arg), &mut output);
    }

    for dir in output {
        clean_dir(&dir.path, dir.ttl_seconds);
    }
}
