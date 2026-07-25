use core::panic;

use crate::ProtoFileBuilder;


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
                    Some(container_name) => container_name,
                    None => crate::consts::DIOXUS_DOCKER_IMAGE_DEFAULT,
                };

                let mut contents = format!("FROM {container_name}\n");
                push_copy_files(&mut contents, copy_files);
                let after = crate::generators::generate_dioxus_fullstack_docker_file(ff_mpeg, service_name);

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
    ci_with_protoc: bool,
    proto_file_builder: Option<ProtoFileBuilder>,
    compile_time_secrets: Vec<(&'static str, &'static str)>,
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
            image_name: crate::consts::DEFAULT_DOCKER_IMAGE_NAME,
            proto_file_builder: None,
            ci_with_protoc: false,
            compile_time_secrets: Default::default(),
        }
    }

    /// Bakes a GitHub secret into the binary at compile time.
    ///
    /// The secret is exported inside the `Build` step of the generated workflow only, so it is
    /// visible to `cargo build --release` on the runner and to nothing else. It is not added to
    /// the Dockerfile, not passed as a `--build-arg` and not put into any job level `env:`,
    /// therefore it does not exist in the produced image at runtime.
    ///
    /// The name of the GitHub secret is used as the name of the env variable.
    /// Use [`Self::add_compile_time_secret_as`] if they have to differ.
    ///
    /// [`Self::build`] bakes the value into the binary itself - read it in the code with
    /// `option_env!("NAME")`. If the value is not injected into the build, `build.rs` writes a
    /// warning into the console; during the release build on the GitHub runner it fails the
    /// build instead.
    pub fn add_compile_time_secret(self, secret_name: &'static str) -> Self {
        self.add_compile_time_secret_as(secret_name, secret_name)
    }

    /// Same as [`Self::add_compile_time_secret`] but the env variable available during the
    /// compilation is named differently from the GitHub secret.
    ///
    /// * `secret_name` - the name of the `secrets.*` entry in GitHub Actions;
    /// * `env_var_name` - the name of the env variable which is exported before `cargo build`.
    pub fn add_compile_time_secret_as(
        mut self,
        secret_name: &'static str,
        env_var_name: &'static str,
    ) -> Self {
        panic_if_bad_name("Github secret name", secret_name);
        panic_if_bad_name("Compile time env variable name", env_var_name);
        self.compile_time_secrets.push((secret_name, env_var_name));
        self
    }

    pub fn add_proto_files_path(mut self, path: &'static str) -> Self {
        self.proto_file_builder = Some(ProtoFileBuilder::new(path));
        self.ci_with_protoc = true;
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

    pub fn ci_with_protoc(mut self) -> Self {
        self.ci_with_protoc = true;
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
                    .unwrap_or(crate::consts::DIOXUS_DOCKER_IMAGE_DEFAULT),
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
                    let docker_image = resolved_docker_image.unwrap_or(crate::consts::DIOXUS_DOCKER_IMAGE_DEFAULT);
                    generate_github_release_dioxus_file(
                        self.service_name,
                        docker_image,
                        self.image_name,
                        self.compile_time_secrets.as_slice(),
                    )
                }
                _ => generate_github_release_file(
                    self.with_ff_mpeg,
                    self.image_name,
                    Some(self.ci_with_protoc),
                    self.compile_time_secrets.as_slice(),
                ),
            }
        }

        if self.ci_test {
            generate_github_test_file();
        }

        bake_declared_compile_time_secrets(self.compile_time_secrets.as_slice());
    }
}

/// Bakes every declared secret into the binary and complains if the value is not injected.
///
/// Locally it is only a `cargo:warning` - the developer has to be able to build the project
/// without having the production secrets at hand. During the release build on the GitHub runner
/// the severity goes up to a hard failure: a released binary without a baked secret is broken
/// anyway, and it is much better to see it as a red `Build` step than at runtime.
fn bake_declared_compile_time_secrets(compile_time_secrets: &[(&'static str, &'static str)]) {
    for (secret_name, env_var_name) in compile_time_secrets {
        if crate::bake_compile_time_secret_value(env_var_name) {
            continue;
        }

        let secret_description = if secret_name == env_var_name {
            format!("'{}'", env_var_name)
        } else {
            format!("'{}' (github secret '{}')", env_var_name, secret_name)
        };

        println!(
            "cargo:warning=Compile time secret {} is declared in the CI workflow but is not injected into this build. option_env!(\"{}\") is going to be None",
            secret_description, env_var_name
        );

        if is_github_release_build() {
            panic!(
                "Compile time secret {} is declared in the CI workflow but is not injected into the build. Create the secret in Github: Settings -> Secrets and variables -> Actions -> New repository secret with the name '{}'",
                secret_description, secret_name
            );
        }
    }
}

/// The release workflow is triggered by a tag, so a tag build on the runner is the build which
/// produces the image. Any other build on the runner - the test workflow for instance - does not
/// export the secrets and must not be broken by them.
fn is_github_release_build() -> bool {
    println!("cargo:rerun-if-env-changed=GITHUB_ACTIONS");
    println!("cargo:rerun-if-env-changed=GITHUB_REF_TYPE");

    if std::env::var("GITHUB_ACTIONS").unwrap_or_default() != "true" {
        return false;
    }

    std::env::var("GITHUB_REF_TYPE").unwrap_or_default() == "tag"
}

fn generate_github_release_file(
    with_ff_mpeg: bool,
    image_name: &str,
    with_protoc: Option<bool>,
    compile_time_secrets: &[(&'static str, &'static str)],
) {
    const OPTIONS_SUB_STRING: &'static str = "#Put Options Here";
    let basic_path = format!(".github{}workflows", std::path::MAIN_SEPARATOR);
    let result = std::fs::create_dir_all(basic_path.as_str());

    if let Err(err) = result {
        panic!("Can not create folder: {}. Err: {}", basic_path, err);
    }

    let release_file = format!("{}{}release.yaml", basic_path, std::path::MAIN_SEPARATOR);

    let yaml_content = replace_versions(crate::RELEASE_YAML_CONTENT, with_protoc)
        .replace("${DOCKER_IMAGE_NAME}", image_name);

    let yaml_content = apply_compile_time_secrets(yaml_content, compile_time_secrets);

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

fn generate_github_release_dioxus_file(
    service_name: &str,
    docker_image: &str,
    image_name: &str,
    compile_time_secrets: &[(&'static str, &'static str)],
) {
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
        .replace("${DOCKER_IMAGE_NAME}", image_name)
        .replace("${DIOXUS_DOCKER_IMAGE_NAME}", crate::consts::DIOXUS_DOCKER_IMAGE_DEFAULT);

    let yaml_content = apply_compile_time_secrets(yaml_content, compile_time_secrets);

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
        .replace("${CHECKOUT_VERSION}", crate::consts::CHECKOUT_VERSION)
        .replace("${RUST_TOOLCHAIN_VERSION}", crate::consts::RUST_TOOLCHAIN_VERSION);

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
#{COMPILE_TIME_SECRETS}
          cargo build --release
"#;

const BUILD_WITH_PROTOC_PART: &'static str = r#"
      - name: Install Protoc and Build
        uses: arduino/setup-protoc@v3
      - run: |
          export GIT_HUB_TOKEN="${{ secrets.PUBLISH_TOKEN }}"
#{COMPILE_TIME_SECRETS}
          cargo build --release
"#;

const COMPILE_TIME_SECRETS_PLACEHOLDER: &'static str = "#{COMPILE_TIME_SECRETS}";

/// Puts `export <ENV_VAR>="${{ secrets.<SECRET> }}"` lines into the build step of the workflow.
///
/// The exports are generated inside the `run:` block on purpose: this way the value lives only
/// in the shell of the build step and does not leak into the other steps of the job
/// (docker build/push included), so it never reaches the resulting image.
fn apply_compile_time_secrets(
    content: String,
    compile_time_secrets: &[(&'static str, &'static str)],
) -> String {
    let mut exports = String::new();

    for (secret_name, env_var_name) in compile_time_secrets {
        exports.push_str("          export ");
        exports.push_str(env_var_name);
        exports.push_str("=\"${{ secrets.");
        exports.push_str(secret_name);
        exports.push_str(" }}\"\n");
    }

    let placeholder_line = format!("{}\n", COMPILE_TIME_SECRETS_PLACEHOLDER);

    content
        .replace(placeholder_line.as_str(), exports.as_str())
        .replace(COMPILE_TIME_SECRETS_PLACEHOLDER, exports.trim_end())
}

fn panic_if_bad_name(name_of: &str, value: &str) {
    let valid = !value.is_empty()
        && !value.starts_with(|c: char| c.is_ascii_digit())
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_');

    if !valid {
        panic!(
            "{} '{}' is invalid. Only latin letters, digits and '_' are allowed and it can not start with a digit",
            name_of, value
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_time_secrets_are_exported_inside_build_step_only() {
        let content = replace_versions(crate::RELEASE_YAML_CONTENT, Some(false));
        let content =
            apply_compile_time_secrets(content, &[("ENCRYPTION_KEY", "ENCRYPTION_KEY"), ("MESH_SECRET", "MESH_KEY")]);

        println!("{}", content);

        assert!(content.contains(
            "          export GIT_HUB_TOKEN=\"${{ secrets.PUBLISH_TOKEN }}\"\n          export ENCRYPTION_KEY=\"${{ secrets.ENCRYPTION_KEY }}\"\n          export MESH_KEY=\"${{ secrets.MESH_SECRET }}\"\n          cargo build --release\n"
        ));

        assert!(!content.contains(COMPILE_TIME_SECRETS_PLACEHOLDER));
        assert!(!content.contains("build-arg"));
    }

    #[test]
    fn test_no_compile_time_secrets() {
        let content = replace_versions(crate::RELEASE_YAML_CONTENT, Some(true));
        let content = apply_compile_time_secrets(content, &[]);

        println!("{}", content);

        assert!(content.contains(
            "          export GIT_HUB_TOKEN=\"${{ secrets.PUBLISH_TOKEN }}\"\n          cargo build --release\n"
        ));
        assert!(!content.contains(COMPILE_TIME_SECRETS_PLACEHOLDER));
    }

    #[test]
    fn test_dioxus_compile_time_secrets() {
        let content = replace_versions(crate::RELEASE_DIOXUS_YAML_CONTENT, None);
        let content = apply_compile_time_secrets(content, &[("ENCRYPTION_KEY", "ENCRYPTION_KEY")]);

        println!("{}", content);

        assert!(content.contains(
            "      - run: |\n          export ENCRYPTION_KEY=\"${{ secrets.ENCRYPTION_KEY }}\"\n          dx bundle --web --release\n"
        ));
    }
}
