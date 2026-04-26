

pub fn generate_dioxus_fullstack_docker_file(ff_mpeg: &str, service_name: &str)->String{
format!("{ff_mpeg}\nENV PORT=9001\nENV IP=0.0.0.0\n\nCOPY ./target/dx/{service_name}/release/web /target/dx/{service_name}/release/web\n\nRUN chmod +x /target/dx/{service_name}/release/web/server\nWORKDIR /target/dx/{service_name}/release/web/\nENTRYPOINT [\"./server\"]")
}