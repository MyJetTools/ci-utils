pub mod js;

use std::path::Path;

use tonic_build::Builder;

pub extern crate tonic_build;

pub fn sync_and_build_proto_file(url_resource: &str, proto_file_name: &str) {
    let proto_path_and_file = prepare_proto_files(url_resource, proto_file_name);

    tonic_build::compile_protos(proto_path_and_file.as_str()).unwrap();
    println!("Proto file {} is compiled", proto_file_name);
}

pub fn sync_and_build_proto_file_from_private_github_repo(
    repo_owner_name: &str,
    repo_name: &str,
    file_path: &str,
) {
    let proto_path_and_file =
        prepare_proto_files_from_private_github(repo_owner_name, repo_name, file_path);

    //    tonic_build::compile_protos(proto_path_and_file.as_str()).unwrap();
    //    println!("Proto file {} is compiled", file_path);
}

pub fn build_proto_from_file(path: &str) {
    tonic_build::compile_protos(path).unwrap();
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

    builder(tonic_build::configure())
        .compile(&[proto_path], &[proto_dir])
        .unwrap();
    println!("Proto file {} is compiled", proto_file_name);
}

fn prepare_proto_files(url_resource: &str, proto_file_name: &str) -> String {
    let url = format!("{}{}", url_resource, proto_file_name);

    let response = if let Ok(git_hub_token) = std::env::var("GIT_HUB_TOKEN") {
        let client = reqwest::blocking::Client::new();
        client
            .get(url)
            .header("Authorization", format!("token {}", git_hub_token))
            .send()
            .unwrap()
    } else {
        reqwest::blocking::get(url).unwrap()
    };

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

fn prepare_proto_files_from_private_github(
    repo_owner: &str,
    repo_name: &str,
    file_path: &str,
) -> String {
    let url = format!("https://api.github.com/repos/{repo_owner}/{repo_name}/contents/{file_path}");

    let response = if let Ok(git_hub_token) = std::env::var("GIT_HUB_TOKEN") {
        let client = reqwest::blocking::Client::new();
        client
            .get(url.as_str())
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "RustCiBuilder")
            .header("Authorization", format!("Bearer {}", git_hub_token))
            .header("X-GitHub-Api-Version", "2022-11-28")
            .send()
            .unwrap()
    } else {
        panic!("Please set GIT_HUB_TOKEN environment variable");
    };

    if !response.status().is_success() {
        panic!(
            "Failed to download proto file from {}. Http Status is: {}",
            url,
            response.status()
        );
    }

    let content = response.text().unwrap();

    let json_value = serde_json::from_str::<serde_json::Value>(content.as_str()).unwrap();

    let map = json_value.as_object().unwrap();

    let content = map.get("content").unwrap();

    let mut proto_file_content = String::new();

    use base64::Engine;

    for itm in content.as_str().unwrap().split("\n") {
        let line = base64::engine::general_purpose::STANDARD
            .decode(itm)
            .unwrap();
        proto_file_content.push_str(String::from_utf8(line).unwrap().as_str());
    }

    let proto_file_name = file_path.split("/").last().unwrap();

    let proto_path_and_file = format!("proto{}{}", std::path::MAIN_SEPARATOR, proto_file_name);

    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(proto_path_and_file.as_str())
        .unwrap();

    std::io::Write::write_all(&mut f, proto_file_content.as_bytes()).unwrap();
    std::io::Write::flush(&mut f).unwrap();

    proto_path_and_file
}
