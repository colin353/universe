#[macro_use]
extern crate tmpl;

use std::collections::HashMap;

static COMPONENT: &str = include_str!("templates/component.js");

#[derive(Debug)]
pub enum CompileError {
    InvalidFilename(String),
    HTMLParsingError(String),
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
    input_css: String,
    symbols: Vec<String>,
    html_in_js: String,
    mutations: Vec<String>,
    refs: Vec<(String, String)>,
    symbol_to_mutations: HashMap<String, Vec<usize>>,
    props: String,
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
            input_css: String::new(),
            symbols: Vec::new(),
            html_in_js: String::new(),
            mutations: Vec::new(),
            refs: Vec::new(),
            symbol_to_mutations: HashMap::new(),
            props: String::new(),
        }
    }

    pub fn log_error(&mut self, err: CompileError) {
        self.errors.push(err);
    }

    pub fn compile(&mut self, input_filename: &str) {
        if !self.extract_file_data(input_filename) {
            return;
        }

        if !self.compile_javascript() {
            return;
        }

        if !self.compile_html() {
            return;
        }

        self.result = tmpl::apply(
            COMPONENT,
            &content!(
                "javascript" => &self.input_javascript,
                "component_name" => &self.component_name,
                "class_name" => &self.class_name,
                "html" => &self.html_in_js,
                "props" => &self.props,
                "css" => &self.input_css;
                "symbols" => self.symbol_to_mutations.iter().map(|(symbol, mutations)| {
                    content!(
                        "name" => symbol,
                        "mutations" => mutations.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",")
                    )
                }).collect(),
                "mutations" => self.mutations.iter().enumerate().map(|(idx, code)| {
                    content!(
                        "idx" => idx,
                        "code" => code
                    )
                }).collect(),
                "refs" => self.refs.iter().map(|(tagname, refname)| {
                    content!(
                        "refname" => refname,
                        "tagname" => tagname
                    )
                }).collect()
            ),
        );
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

        let mut first = true;
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
            first = false;

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

        match self
            .filesystem
            .read_to_string(format!("{}/{}.css", parent.display(), filename))
        {
            Ok(s) => self.input_css = s,
            Err(_) => (),
        }

        true
    }

    fn compile_javascript(&mut self) -> bool {
        // Extract possible props declaration
        let mut lines = self.input_javascript.lines().map(|x| x.trim()).peekable();
        let marker = "const attributes = [";
        while let Some(line) = lines.peek() {
            let mut start = 0;
            if line.starts_with(marker) {
                start = marker.len();
                while let Some(line) = lines.peek() {
                    if let Some(idx) = line.find("]") {
                        self.props += &line[start..idx].trim();
                        break;
                    } else {
                        self.props += &line[start..].trim();
                    }
                    start = 0;
                    lines.next();
                }
            }
            lines.next();
        }

        true
    }

    fn compile_html(&mut self) -> bool {
        let mut elements = match htmlc::parse(&self.input_html) {
            Ok(e) => e,
            Err(s) => {
                self.log_error(CompileError::HTMLParsingError(s));
                return false;
            }
        };

        let mut mutators = Vec::new();

        for element in &mut elements {
            self.html_in_js
                .push_str(&element.to_js("this.shadow", "true"));
            self.html_in_js.push('\n');

            for mutator in element.get_mutators("this.shadow") {
                mutators.push(mutator);
            }
        }

        let mut observed = HashMap::new();
        for mutator in mutators {
            self.mutations.push(mutator.operation);

            for dep in &mutator.inputs {
                if let Some(symbol_idx) = observed.get_mut(dep) {
                    let entry = self.symbol_to_mutations.get_mut(dep).unwrap();
                    entry.push(self.mutations.len() - 1);
                } else {
                    self.symbols.push(dep.to_string());
                    observed.insert(dep.to_string(), self.symbols.len() - 1);
                    self.symbol_to_mutations
                        .insert(dep.to_string(), vec![self.mutations.len() - 1]);
                }
            }
        }

        for element in &elements {
            self.extract_refs(element);
        }

        true
    }

    fn extract_refs(&mut self, element: &htmlc::HTMLElement) {
        for (key, value) in &element.attributes {
            if key == "ref" {
                self.refs.push((element.name.clone(), value.to_owned()));
            }
        }

        for child in &element.children {
            self.extract_refs(child);
        }
    }
}
