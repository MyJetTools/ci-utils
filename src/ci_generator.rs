use core::panic;

use crate::ProtoFileBuilder;

const CHECKOUT_VERSION: &str = "v6.0.2";
const RUST_TOOLCHAIN_VERSION: &str = "v1.15.2";
const DIOXUS_VERSION: &str = "0.7.2";
const DIOXUS_DOCKER_IMAGE_DEFAULT: &str = "myjettools/dioxus-docker:0.7.2";
const DEFAULT_DOCKER_IMAGE_NAME: &str = "ghcr.io/${{ github.repository }}";

#[derive(Clone, Copy)]
pub enum DockerFileType {
    Basic,
    DioxusFullStack,
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
            DockerFileType::DioxusFullStack => {
                let container_name = match container_name {
                    Some(container_name) => container_name.to_string(),
                    None => format!("myjettools/dioxus-docker:{}", DIOXUS_VERSION).to_string(),
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
    ci_test: bool,
    image_name: &'static str,
    proto_file_builder: Option<ProtoFileBuilder>,
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
            ci_test: false,
            image_name: DEFAULT_DOCKER_IMAGE_NAME,
            proto_file_builder: None,
        }
    }

    pub fn add_proto_files_path(mut self, path: &'static str) -> Self {
        self.proto_file_builder = Some(ProtoFileBuilder::new(path));
        self
    }

    pub fn add_proto_file(mut self, proto_file_name: &'static str) -> Self {
        let builder = self.proto_file_builder.take();

        let Some(builder) = builder else {
            panic!("Specify proto files path first");
        };

        self.proto_file_builder = Some(builder.sync_and_build(proto_file_name));

        self
    }

    pub fn add_docker_copy_file(mut self, from_file: &'static str, to_file: &'static str) -> Self {
        self.docker_copy.push((from_file, to_file));
        self
    }

    pub fn set_docker_container_name_build_from(mut self, container_name: &'static str) -> Self {
        self.docker_container_name = Some(container_name);
        self
    }

    pub fn set_docker_image_name(mut self, image_name: &'static str) -> Self {
        self.image_name = image_name;
        self
    }

    pub fn as_basic_service(mut self) -> Self {
        self.docker_file = Some(DockerFileType::Basic);
        self
    }

    pub fn as_dioxus_fullstack_service(mut self) -> Self {
        self.docker_file = Some(DockerFileType::DioxusFullStack);
        self
    }

    pub fn with_ci_test(mut self) -> Self {
        self.ci_test = true;
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
        let resolved_docker_image = match self.docker_file {
            Some(DockerFileType::DioxusFullStack) => Some(
                self.docker_container_name
                    .unwrap_or(DIOXUS_DOCKER_IMAGE_DEFAULT),
            ),
            _ => self.docker_container_name,
        };

        if let Some(docker_file) = self.docker_file {
            docker_file.generate_docker_file(
                self.service_name,
                self.with_ff_mpeg,
                resolved_docker_image,
                self.docker_copy.as_slice(),
            );
        }

        if self.generate_github_ci_file {
            match self.docker_file {
                Some(DockerFileType::DioxusFullStack) => {
                    let docker_image = resolved_docker_image.unwrap_or(DIOXUS_DOCKER_IMAGE_DEFAULT);
                    generate_github_release_dioxus_file(
                        self.service_name,
                        docker_image,
                        self.image_name,
                    )
                }
                _ => generate_github_release_file(
                    self.with_ff_mpeg,
                    self.image_name,
                    Some(self.proto_file_builder.is_some()),
                ),
            }
        }

        if self.ci_test {
            generate_github_test_file();
        }
    }
}

fn generate_github_release_file(with_ff_mpeg: bool, image_name: &str, with_protoc: Option<bool>) {
    const OPTIONS_SUB_STRING: &'static str = "#Put Options Here";
    let basic_path = format!(".github{}workflows", std::path::MAIN_SEPARATOR);
    let result = std::fs::create_dir_all(basic_path.as_str());

    if let Err(err) = result {
        panic!("Can not create folder: {}. Err: {}", basic_path, err);
    }

    let release_file = format!("{}{}release.yaml", basic_path, std::path::MAIN_SEPARATOR);

    let yaml_content = replace_versions(crate::RELEASE_YAML_CONTENT, with_protoc)
        .replace("${DOCKER_IMAGE_NAME}", image_name);

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

fn generate_github_release_dioxus_file(service_name: &str, docker_image: &str, image_name: &str) {
    let basic_path = format!(".github{}workflows", std::path::MAIN_SEPARATOR);
    if let Err(err) = std::fs::create_dir_all(basic_path.as_str()) {
        panic!("Can not create folder: {}. Err: {}", basic_path, err);
    }

    let release_file = format!("{}{}release.yaml", basic_path, std::path::MAIN_SEPARATOR);

    let dioxus_version = docker_image
        .rsplit_once(':')
        .map(|(_, ver)| ver)
        .unwrap_or("latest");

    let yaml_content = replace_versions(crate::RELEASE_DIOXUS_YAML_CONTENT, None)
        .replace("${SERVICE_NAME}", service_name)
        .replace("${DIOXUS_VERSION}", dioxus_version)
        .replace("${DOCKER_IMAGE_NAME}", image_name);

    if let Err(err) = std::fs::write(release_file.as_str(), yaml_content) {
        panic!(
            "Can not create file: {}. Err: {}",
            release_file.as_str(),
            err
        );
    }
}

fn generate_github_test_file() {
    let basic_path = format!(".github{}workflows", std::path::MAIN_SEPARATOR);
    if let Err(err) = std::fs::create_dir_all(basic_path.as_str()) {
        panic!("Can not create folder: {}. Err: {}", basic_path, err);
    }

    let test_file = format!("{}{}test.yml", basic_path, std::path::MAIN_SEPARATOR);
    let test_content = replace_versions(crate::TEST_YAML_CONTENT, None);
    if let Err(err) = std::fs::write(test_file.as_str(), test_content) {
        panic!("Can not create file: {}. Err: {}", test_file.as_str(), err);
    }
}

fn replace_versions(content: &str, with_protoc: Option<bool>) -> String {
    let content = content
        .replace("${CHECKOUT_VERSION}", CHECKOUT_VERSION)
        .replace("${RUST_TOOLCHAIN_VERSION}", RUST_TOOLCHAIN_VERSION);

    match with_protoc {
        Some(with_protoc) => {
            if with_protoc {
                content.replace("#{BUILD}", BUILD_WITH_PROTOC_PART)
            } else {
                content.replace("#{BUILD}", BUILD_PART)
            }
        }

        None => content,
    }
}

const BUILD_PART: &'static str = r#"
      - name: Build
        run: |
          export GIT_HUB_TOKEN="${{ secrets.PUBLISH_TOKEN }}"
          cargo build --release
"#;

const BUILD_WITH_PROTOC_PART: &'static str = r#"
      - name: Install Protoc and Build
        uses: arduino/setup-protoc@v3        
      - run: |
          export GIT_HUB_TOKEN="${{ secrets.PUBLISH_TOKEN }}"
          cargo build --release
"#;
