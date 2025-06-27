#[derive(Clone, Copy)]
pub enum DockerFileType {
    Basic,
}

impl DockerFileType {
    pub fn generate_docker_file(&self, service_name: &'static str) {
        match self {
            DockerFileType::Basic => {
                let contents = format!("FROM ubuntu:22.04\nCOPY ./target/release/{service_name} ./target/release/{service_name}\nENTRYPOINT [\"./target/release/{service_name}\"]");
                std::fs::write("Dockerfile", contents).unwrap();
            }
        }
    }
}

pub struct CiGenerator {
    service_name: &'static str,
    docker_file: Option<DockerFileType>,
}

impl CiGenerator {
    pub fn new(service_name: &'static str) -> Self {
        Self {
            service_name,
            docker_file: None,
        }
    }

    pub fn as_basic_service(mut self) -> Self {
        self.docker_file = Some(DockerFileType::Basic);
        self
    }

    pub fn build(self) {
        if let Some(docker_file) = self.docker_file {
            docker_file.generate_docker_file(self.service_name);
        }
    }
}
