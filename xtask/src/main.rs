use std::{
    path::Path,
    process::{Command, ExitCode},
};
fn main() -> ExitCode {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let candidates: &[(&str, &[&str])] = if cfg!(windows) {
        &[("py", &["-3"]), ("python", &[])]
    } else {
        &[("python3", &[])]
    };
    for (program, prefix) in candidates {
        match Command::new(program)
            .env("PYTHONUTF8", "1")
            .args(*prefix)
            .arg(root.join("tools/dev.py"))
            .args(&args)
            .current_dir(root)
            .status()
        {
            Ok(status) => return ExitCode::from(status.code().unwrap_or(1).clamp(0, 255) as u8),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                eprintln!("Cannot run developer command: {error}");
                return ExitCode::FAILURE;
            }
        }
    }
    eprintln!("Python 3.10+ is required for repository tooling; see CONTRIBUTING.md");
    ExitCode::FAILURE
}
