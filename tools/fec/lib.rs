use std::collections::HashMap;

#[derive(Debug)]
pub enum CompileError {
    InvalidFilename(String),
}

pub struct FECompiler {
    pub errors: Vec<CompileError>,
    pub result: String,
    filesystem: fs::FSAccessor,

    // Extracted properties
    component_name: String,
    class_name: String,
    input_javascript: String,
    input_html: String,
}

impl FECompiler {
    pub fn new() -> Self {
        FECompiler {
            errors: Vec::new(),
            result: String::new(),
            filesystem: fs::FSAccessor::new(),

            // Extracted properties
            component_name: String::new(),
            class_name: String::new(),
            input_javascript: String::new(),
            input_html: String::new(),
        }
    }

    pub fn log_error(&mut self, err: CompileError) {
        self.errors.push(err);
    }

    pub fn compile(&mut self, input_filename: &str) {
        if !self.extract_file_data(input_filename) {
            return;
        }
    }

    pub fn success(&self) -> bool {
        self.errors.len() == 0
    }

    fn extract_file_data(&mut self, input_filename: &str) -> bool {
        let path = std::path::Path::new(input_filename);

        let filename = match path.file_stem() {
            Some(s) => s.to_str().unwrap(),
            None => {
                self.log_error(CompileError::InvalidFilename(format!(
                    "Invalid input filename: `{:?}`",
                    path
                )));
                return true;
            }
        };

        let mut first = false;
        let mut has_fatal_errors = false;
        let mut has_underscore = false;
        if filename.ends_with("_") {
            self.log_error(CompileError::InvalidFilename(String::from(
                "filename must end with an alphanumeric character",
            )));
            has_fatal_errors = true;
        }
        for ch in filename.chars() {
            if first && !ch.is_ascii_alphabetic() {
                self.log_error(CompileError::InvalidFilename(String::from(
                    "filename must start with a letter",
                )));
                has_fatal_errors = true;
            }

            if ch == '_' {
                has_underscore = true;
            }

            if !ch.is_ascii_alphanumeric() && ch != '_' {
                self.log_error(CompileError::InvalidFilename(format!(
                    "filename contains invalid character: `{}`",
                    ch
                )));
                has_fatal_errors = true;
            }
        }

        if !has_underscore {
            self.log_error(CompileError::InvalidFilename(
                "filename must contain at least one underscore".to_string(),
            ));
            has_fatal_errors = true;
        }

        if has_fatal_errors {
            return false;
        }

        self.component_name = filename.to_lowercase().replace("_", "-");
        self.class_name = filename
            .split("_")
            .map(|s| {
                s.chars()
                    .enumerate()
                    .map(|(idx, ch)| match idx {
                        0 => ch.to_ascii_uppercase(),
                        _ => ch.to_ascii_lowercase(),
                    })
                    .collect::<String>()
            })
            .collect();

        match self.filesystem.read_to_string(path) {
            Ok(s) => self.input_javascript = s,
            Err(_) => {
                self.log_error(CompileError::InvalidFilename(format!(
                    "could not read input file: {:?}",
                    path
                )));
                return false;
            }
        }

        let parent = path.parent().unwrap();
        match self
            .filesystem
            .read_to_string(format!("{}/{}.html", parent.display(), filename))
        {
            Ok(s) => self.input_html = s,
            Err(_) => (),
        }

        true
    }
}
