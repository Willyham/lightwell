fn main() {
    let mut failed = false;
    for name in std::env::args_os().skip(1) {
        let path = std::path::Path::new(&name);
        let id = path.file_name().unwrap_or_default().to_string_lossy();
        match lightwell_s0_probes::open(path) {
            Ok(photo) => println!(
                "{}",
                serde_json::json!({"fixture": id, "status":"decoded", "width":photo.width, "height":photo.height, "rgba_bytes":photo.rgba.len(), "decode_ms":photo.decode_ms})
            ),
            Err(error) => {
                failed = true;
                println!(
                    "{}",
                    serde_json::json!({"fixture":id,"status":"error","error":error})
                );
            }
        }
    }
    if failed {
        std::process::exit(1);
    }
}
