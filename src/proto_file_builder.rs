pub struct ProtoFileBuilder {
    base_url: &'static str,
    skip_syncing: bool,
}

impl ProtoFileBuilder {
    pub fn new(base_url: &'static str) -> Self {
        Self {
            base_url,
            skip_syncing: false,
        }
    }

    pub fn skip_syncing(mut self) -> Self {
        self.skip_syncing = true;
        self
    }

    pub fn sync_and_build(self, proto_file_name: &'static str) -> Self {
        let proto_path_and_file =
            crate::prepare_proto_files(self.base_url, proto_file_name, self.skip_syncing);

        crate::compile_protos(&proto_path_and_file);
        self
    }
}
