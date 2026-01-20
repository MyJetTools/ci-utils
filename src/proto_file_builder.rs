pub struct ProtoFileBuilder {
    base_url_or_path: &'static str,
    skip_syncing: bool,
}

impl ProtoFileBuilder {
    pub fn new(base_url_or_path: &'static str) -> Self {
        Self {
            base_url_or_path,
            skip_syncing: false,
        }
    }

    pub fn skip_syncing(mut self) -> Self {
        self.skip_syncing = true;
        self
    }

    pub fn sync_and_build(self, proto_file_name: &'static str) -> Self {
        let proto_file_name = if self.base_url_or_path.starts_with("http") {
            crate::prepare_proto_files(self.base_url_or_path, proto_file_name, self.skip_syncing)
        } else {
            copy_files(self.base_url_or_path, proto_file_name)
        };

        crate::compile_protos(proto_file_name.as_str());
        self
    }
}

fn copy_files(from_path: &str, proto_file_name: &str) -> String {
    let proto_file_content = match std::fs::read(from_path) {
        Ok(file_content) => file_content,
        Err(err) => {
            panic!(
                "Can not open proto source file '{}'. Err: {:?}",
                from_path, err
            )
        }
    };

    super::proto_file_utils::write_proto_file(proto_file_name, proto_file_content.as_slice())
}
