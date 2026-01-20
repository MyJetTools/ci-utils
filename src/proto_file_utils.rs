pub fn write_proto_file(proto_file_name: &str, content: &[u8]) -> String {
    let dest_file_path = format_proto_file_name(proto_file_name);
    write_file(&dest_file_path, content);
    dest_file_path
}

pub fn write_file(file_path: &str, content: &[u8]) {
    let f = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(file_path);

    if let Err(e) = &f {
        panic!("Failed to open file {}. Err: {}", file_path, e);
    }

    let mut f = f.unwrap();

    let write_result = std::io::Write::write_all(&mut f, content);

    if let Err(e) = &write_result {
        panic!("Failed to write to file {}. Err: {}", file_path, e);
    }
    let result = std::io::Write::flush(&mut f);

    if let Err(e) = &result {
        panic!("Failed to flush to file {}. Err: {}", file_path, e);
    }
}

pub fn format_proto_file_name(proto_file_name: &str) -> String {
    format!("proto{}{}", std::path::MAIN_SEPARATOR, proto_file_name)
}
