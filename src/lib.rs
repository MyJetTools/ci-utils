pub mod js;
mod proto_file_builder;
pub use proto_file_builder::*;
pub extern crate tonic_prost_build;
pub mod ci_generator;
pub mod css;
mod proto_file_utils;

const RELEASE_YAML_CONTENT: &str = std::include_str!("../release.yml");
const TEST_YAML_CONTENT: &str = std::include_str!("../test.yml");
const RELEASE_DIOXUS_YAML_CONTENT: &str = std::include_str!("../release-dioxus.yaml");
const FFMPEG_OPTION: &str = std::include_str!("../ffmpeg.yaml");

pub fn compile_protos(proto_file_name: &str) {
    tonic_prost_build::configure()
        .protoc_arg("--experimental_allow_proto3_optional")
        .compile_protos(&[proto_file_name], &["proto"])
        .unwrap();
}

pub fn sync_and_build_proto_file(url_resource: &str, proto_file_name: &str) {
    let proto_path_and_file = prepare_proto_files(url_resource, proto_file_name, false);

    //tonic_build::compile_protos(proto_path_and_file.as_str()).unwrap();

    compile_protos(&proto_path_and_file);
    println!("Proto file {} is compiled", proto_file_name);
}

pub fn download_file(url_resource: &str, dest_path: &str) {
    let response = reqwest::blocking::get(url_resource).unwrap();

    if !response.status().is_success() {
        panic!(
            "Failed to download file {}. Http Status is: {}",
            url_resource,
            response.status()
        );
    }

    let content = response.text().unwrap();
    crate::proto_file_utils::write_file(dest_path, content.as_bytes());
}

fn prepare_proto_files(url_resource: &str, proto_file_name: &str, skip_syncing: bool) -> String {
    let url = if url_resource.ends_with("/") {
        format!("{}{}", url_resource, proto_file_name)
    } else {
        format!("{}/{}", url_resource, proto_file_name)
    };

    if skip_syncing {
        return crate::proto_file_utils::format_proto_file_name(proto_file_name);
    }

    let response = reqwest::blocking::get(url.as_str()).unwrap();

    if !response.status().is_success() {
        panic!(
            "Failed to download proto file '{}'. Http Status is: {:?}. Using token: false",
            url, response
        );
    }
    let content = response.text().unwrap();

    println!("Proto file {} is downloaded", proto_file_name);

    crate::proto_file_utils::write_proto_file(proto_file_name, content.as_bytes())
}
