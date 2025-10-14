#[derive(Clone, Copy)]
pub enum DockerFileType {
    Basic,
}

impl DockerFileType {
    pub fn generate_docker_file(&self, service_name: &'static str, with_ff_mpeg: bool) {
        let ff_mpeg = if with_ff_mpeg {
            "RUN apt upgrade -y\nRUN apt update\nRUN apt install ffmpeg libavcodec-dev libavformat-dev libavutil-dev libswresample-dev libswscale-dev libavfilter-dev libavdevice-dev -y\n"
        } else {
            ""
        };
        match self {
            DockerFileType::Basic => {
                let contents = format!("FROM ubuntu:22.04\n{ff_mpeg}COPY ./target/release/{service_name} ./target/release/{service_name}\nENTRYPOINT [\"./target/release/{service_name}\"]");
                std::fs::write("Dockerfile", contents).unwrap();
            }
        }
    }
}

pub struct CiGenerator {
    service_name: &'static str,
    docker_file: Option<DockerFileType>,
    generate_github_ci_file: bool,
    with_ff_mpeg: bool,
}

impl CiGenerator {
    pub fn new(service_name: &'static str) -> Self {
        Self {
            service_name,
            docker_file: None,
            generate_github_ci_file: false,
            with_ff_mpeg: false,
        }
    }

    pub fn as_basic_service(mut self) -> Self {
        self.docker_file = Some(DockerFileType::Basic);
        self
    }

    pub fn with_ff_mpeg(mut self) -> Self {
        self.with_ff_mpeg = true;
        self
    }

    pub fn generate_github_ci_file(mut self) -> Self {
        self.generate_github_ci_file = true;
        self
    }

    pub fn build(self) {
        if let Some(docker_file) = self.docker_file {
            docker_file.generate_docker_file(self.service_name, self.with_ff_mpeg);
        }

        if self.generate_github_ci_file {
            generate_github_release_file(self.with_ff_mpeg)
        }
    }
}

fn generate_github_release_file(with_ff_mpeg: bool) {
    const OPTIONS_SUB_STRING: &'static str = "#Put Options Here";
    let basic_path = format!(".github{}workflows", std::path::MAIN_SEPARATOR);
    let result = std::fs::create_dir_all(basic_path.as_str());

    if let Err(err) = result {
        panic!("Can not create folder: {}. Err: {}", basic_path, err);
    }

    let release_file = format!("{}{}release.yml", basic_path, std::path::MAIN_SEPARATOR);

    let yaml_content = crate::RELEASE_YAML_CONTENT;

    let release_file_to_write = if with_ff_mpeg {
        yaml_content.replace(OPTIONS_SUB_STRING, crate::FFMPEG_OPTION)
    } else {
        yaml_content.replace(OPTIONS_SUB_STRING, "")
    };

    let result = std::fs::write(release_file.as_str(), release_file_to_write);

    if let Err(err) = result {
        panic!(
            "Can not create file: {}. Err: {}",
            release_file.as_str(),
            err
        );
    }
}
