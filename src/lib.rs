use std::path::Path;

use tonic_build::Builder;

pub fn sync_and_build_proto_file(url_resource: &str, proto_file_name: &str) {
    let proto_path_and_file = prepare_proto_files(url_resource, proto_file_name);

    tonic_build::compile_protos(proto_path_and_file.as_str()).unwrap();
    println!("Proto file {} is compiled", proto_file_name);
}

pub fn build_proto_from_file(path: &str){
    tonic_build::compile_protos(path).unwrap();
}

pub fn sync_and_build_proto_file_with_builder(
    url_resource: &str,
    proto_file_name: &str,
    builder: impl Fn(Builder) -> Builder,
) {
    let proto_path_and_file = prepare_proto_files(url_resource, proto_file_name);

    let proto_path: &Path = proto_path_and_file.as_ref();

    let proto_dir = proto_path
        .parent()
        .expect("proto file should reside in a directory");

    builder(tonic_build::configure()).compile(&[proto_path], &[proto_dir]).unwrap();
    println!("Proto file {} is compiled", proto_file_name);
}

fn prepare_proto_files(url_resource: &str, proto_file_name: &str) -> String {
    let url = format!("{}{}", url_resource, proto_file_name);
    let response = reqwest::blocking::get(url).unwrap();
    if !response.status().is_success() {
        panic!(
            "Failed to download proto file. Http Status is: {}",
            response.status()
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
