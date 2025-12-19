pub struct CssCompiler {
    directory: &'static str,
    files: Vec<&'static str>,
}

impl CssCompiler {
    pub fn new(directory: &'static str) -> Self {
        Self {
            directory,
            files: vec![],
        }
    }

    pub fn add_file(mut self, file: &'static str) -> Self {
        self.files.push(file);
        self
    }

    pub fn compile(&self, out_file: &str) {
        let mut content = String::new();

        for file in self.files.iter() {
            let file_to_open = if self.directory.ends_with(std::path::MAIN_SEPARATOR) {
                format!("{}{}", self.directory, file)
            } else {
                format!("{}{}{}", self.directory, std::path::MAIN_SEPARATOR, file)
            };

            let content_to_merge = match std::fs::read_to_string(file_to_open.as_str()) {
                Ok(content) => content,
                Err(err) => {
                    panic!("Can not open file '{}'. Error: {:?}", file_to_open, err);
                }
            };

            content.push_str(content_to_merge.as_str());
            content.push_str("\n");
        }

        let current_content = std::fs::read(out_file).unwrap_or_default();

        if current_content.as_slice() != content.as_bytes() {
            if let Err(err) = std::fs::write(out_file, content.as_bytes()) {
                panic!("Can not write file '{}'. Err: {}", out_file, err)
            }
        }
    }
}
