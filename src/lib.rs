pub mod js;
mod proto_file_builder;
pub use proto_file_builder::*;
pub extern crate tonic_prost_build;
pub mod ci_generator;

const RELEASE_YAML_CONTENT: &[u8] = std::include_bytes!("../release.yml");

pub fn compile_protos(proto_file_name: &str) {
    tonic_prost_build::configure()
        .protoc_arg("--experimental_allow_proto3_optional")
        .compile_protos(&[proto_file_name], &["proto"])
        .unwrap();
}

pub fn sync_and_build_proto_file(url_resource: &str, proto_file_name: &str) {
    let proto_path_and_file = prepare_proto_files(url_resource, proto_file_name);

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

    let f = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(dest_path);

    if let Err(e) = &f {
        panic!("Failed to open file {}. Err: {}", dest_path, e);
    }

    let mut f = f.unwrap();

    let write_result = std::io::Write::write_all(&mut f, content.as_bytes());

    if let Err(e) = &write_result {
        panic!("Failed to write to file {}. Err: {}", dest_path, e);
    }
    let result = std::io::Write::flush(&mut f);

    if let Err(e) = &result {
        panic!("Failed to flush to file {}. Err: {}", dest_path, e);
    }
}

fn prepare_proto_files(url_resource: &str, proto_file_name: &str) -> String {
    let url = if url_resource.ends_with("/") {
        format!("{}{}", url_resource, proto_file_name)
    } else {
        format!("{}/{}", url_resource, proto_file_name)
    };

    let response = reqwest::blocking::get(url.as_str()).unwrap();

    if !response.status().is_success() {
        panic!(
            "Failed to download proto file '{}'. Http Status is: {:?}. Using token: false",
            url, response
        );
    }
    let content = response.text().unwrap();

    println!("Proto file {} is downloaded", proto_file_name);

    let proto_path_and_file = format!("proto{}{}", std::path::MAIN_SEPARATOR, proto_file_name);

    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(proto_path_and_file.as_str())
        .unwrap();

    std::io::Write::write_all(&mut f, content.as_bytes()).unwrap();
    std::io::Write::flush(&mut f).unwrap();
    println!("Proto file {} is updated", proto_file_name);

    proto_path_and_file
}
