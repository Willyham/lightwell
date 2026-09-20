use lightwell_core::{OwnerHandle, serve_json_lines};
use std::path::PathBuf;

fn main() {
    if let Err(error) = run() {
        eprintln!(
            "{}",
            serde_json::json!({"error":{"code":error.0,"message":error.1}})
        );
        std::process::exit(1);
    }
}

fn run() -> Result<(), (String, String)> {
    let mut args = std::env::args_os().skip(1);
    let mut catalog: Option<PathBuf> = None;
    while let Some(argument) = args.next() {
        match argument.to_str() {
            Some("--catalog") => {
                catalog = Some(
                    args.next()
                        .ok_or_else(|| ("startup".into(), "--catalog requires a path".into()))?
                        .into(),
                );
            }
            Some("--help") => {
                println!("lightwell-json --catalog CATALOG < requests.jsonl");
                return Ok(());
            }
            _ => return Err(("startup".into(), "unknown argument; use --help".into())),
        }
    }
    let catalog = catalog.ok_or_else(|| ("startup".into(), "--catalog is required".into()))?;
    let (owner, join) =
        OwnerHandle::start(&catalog).map_err(|error| (error.kind.code().into(), error.detail))?;
    let served = serve_json_lines(std::io::stdin().lock(), std::io::stdout().lock(), &owner);
    owner.stop();
    let _ = join.join();
    served.map_err(|error| (error.kind.code().into(), error.detail))
}
