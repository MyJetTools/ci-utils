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
    generate_github_ci_file: bool,
}

impl CiGenerator {
    pub fn new(service_name: &'static str) -> Self {
        Self {
            service_name,
            docker_file: None,
            generate_github_ci_file: false,
        }
    }

    pub fn as_basic_service(mut self) -> Self {
        self.docker_file = Some(DockerFileType::Basic);
        self
    }

    pub fn generate_github_ci_file(mut self) -> Self {
        self.generate_github_ci_file = true;
        self
    }

    pub fn build(self) {
        if let Some(docker_file) = self.docker_file {
            docker_file.generate_docker_file(self.service_name);
        }

        if self.generate_github_ci_file {
            generate_github_release_file()
        }
    }
}

fn generate_github_release_file() {
    let basic_path = format!(".github{}workflows", std::path::MAIN_SEPARATOR);
    let result = std::fs::create_dir_all(basic_path.as_str());

    if let Err(err) = result {
        panic!("Can not create folder: {}. Err: {}", basic_path, err);
    }

    let release_file = format!("{}{}release.yml", basic_path, std::path::MAIN_SEPARATOR);
    let result = std::fs::write(release_file.as_str(), crate::RELEASE_YAML_CONTENT);

    if let Err(err) = result {
        panic!(
            "Can not create file: {}. Err: {}",
            release_file.as_str(),
            err
        );
    }
}
