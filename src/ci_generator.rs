#[derive(Clone, Copy)]
pub enum DockerFileType {
    Basic,
    Dioxus,
}

impl DockerFileType {
    pub fn generate_docker_file(
        &self,
        service_name: &'static str,
        with_ff_mpeg: bool,
        container_name: Option<&str>,
        copy_files: &[(&'static str, &'static str)],
    ) {
        let ff_mpeg = if with_ff_mpeg {
            "RUN apt upgrade -y\nRUN apt update\nRUN apt install ffmpeg libavcodec-dev libavformat-dev libavutil-dev libswresample-dev libswscale-dev libavfilter-dev libavdevice-dev -y\n"
        } else {
            ""
        };
        match self {
            DockerFileType::Basic => {
                let container_name = match container_name {
                    Some(container_name) => container_name,
                    None => "ubuntu:22.04",
                };

                let mut contents = format!("FROM {container_name}\n{ff_mpeg}COPY ./target/release/{service_name} ./target/release/{service_name}\n");
                push_copy_files(&mut contents, copy_files);
                contents
                    .push_str(format!("ENTRYPOINT [\"./target/release/{service_name}\"]").as_str());
                std::fs::write("Dockerfile", contents).unwrap();
            }
            DockerFileType::Dioxus => {
                let container_name = match container_name {
                    Some(container_name) => container_name,
                    None => "myjettools/dioxus-docker:0.7.0",
                };

                let mut contents = format!("FROM {container_name}\n");
                push_copy_files(&mut contents, copy_files);
                let after = format!("{ff_mpeg}\nENV PORT=9001\nENV IP=0.0.0.0\n\nCOPY ./target/dx/{service_name}/release/web /target/dx/{service_name}/release/web\n\nRUN chmod +x /target/dx/{service_name}/release/web/{service_name}\nWORKDIR /target/dx/{service_name}/release/web/\nENTRYPOINT [\"./{service_name}\"]");

                contents.push_str(after.as_str());
                std::fs::write("Dockerfile", contents).unwrap();
            }
        }
    }
}

fn push_copy_files(out: &mut String, copy: &[(&'static str, &'static str)]) {
    for itm in copy {
        out.push_str("COPY ");
        out.push_str(itm.0);
        out.push(' ');
        out.push_str(itm.1);
        out.push('\n');
    }
}

pub struct CiGenerator {
    service_name: &'static str,
    docker_file: Option<DockerFileType>,
    generate_github_ci_file: bool,
    with_ff_mpeg: bool,
    docker_copy: Vec<(&'static str, &'static str)>,
    docker_container_name: Option<&'static str>,
}

impl CiGenerator {
    pub fn new(service_name: &'static str) -> Self {
        Self {
            service_name,
            docker_file: None,
            generate_github_ci_file: false,
            with_ff_mpeg: false,
            docker_copy: Default::default(),
            docker_container_name: Default::default(),
        }
    }

    pub fn add_docker_copy_file(mut self, from_file: &'static str, to_file: &'static str) -> Self {
        self.docker_copy.push((from_file, to_file));
        self
    }

    pub fn set_docker_container_name(mut self, container_name: &'static str) -> Self {
        self.docker_container_name = Some(container_name);
        self
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
            docker_file.generate_docker_file(
                self.service_name,
                self.with_ff_mpeg,
                self.docker_container_name,
                self.docker_copy.as_slice(),
            );
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
