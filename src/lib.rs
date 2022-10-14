pub fn sync_and_build_proto_file(url_resource: &str, proto_file_name: &str) {
    let url = format!("{}{}", url_resource, proto_file_name);
    let response = reqwest::blocking::get(url).unwrap();
    let content = response.text().unwrap();

    println!("Proto file {} is downloaded", proto_file_name);

    let proto_path_and_file = format!("proto/{}", proto_file_name);

    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(proto_path_and_file.as_str())
        .unwrap();

    std::io::Write::write_all(&mut f, content.as_bytes()).unwrap();
    std::io::Write::flush(&mut f).unwrap();

    println!("Proto file {} is updated", proto_file_name);

    tonic_build::compile_protos(proto_path_and_file.as_str()).unwrap();
    println!("Proto file {} is compiled", proto_file_name);
}
