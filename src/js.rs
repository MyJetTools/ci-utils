use std::io::Write;

pub fn merge_js_files(js_files: &[&str], out_file_name: &str) {
    let mut out_file = std::fs::File::create(out_file_name).unwrap();

    for file_name in js_files {
        if file_name.ends_with(".js") {
            let content = std::fs::read_to_string(format!("JavaScript/{}", file_name)).unwrap();

            out_file
                .write_all(format!("// {}\n", file_name).as_bytes())
                .unwrap();

            for line in content.split('\n') {
                if line.trim().starts_with("//") {
                    continue;
                }

                out_file
                    .write_all(format!("{}\n", line).as_bytes())
                    .unwrap();
            }

            out_file.write_all("\n".as_bytes()).unwrap();
        }
    }
}
