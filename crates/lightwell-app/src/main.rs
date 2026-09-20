mod diagnostics;
mod editor_app;
mod paths;
use diagnostics::Diagnostics;
use std::{collections::VecDeque, path::PathBuf};

#[derive(Clone, Default)]
struct Config {
    files: VecDeque<PathBuf>,
    evidence: Option<PathBuf>,
    size: Option<(f32, f32)>,
    data_root: Option<PathBuf>,
    catalog: Option<PathBuf>,
    diagnostics: Option<Diagnostics>,
    run_id: String,
}

impl Config {
    /// Structured events are wanted when a data root or evidence directory was requested, even
    /// if the log file could not be created; they then fall back to stderr.
    fn wants_events(&self) -> bool {
        self.evidence.is_some() || self.data_root.is_some()
    }
}

fn arguments() -> Result<Config, String> {
    let mut config = Config::default();
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--open") => config
                .files
                .push_back(args.next().ok_or("--open requires a path")?.into()),
            Some("--evidence-dir") => {
                config.evidence = Some(args.next().ok_or("--evidence-dir requires a path")?.into())
            }
            Some("--data-root") => {
                config.data_root = Some(args.next().ok_or("--data-root requires a path")?.into());
            }
            Some("--catalog") => {
                config.catalog = Some(args.next().ok_or("--catalog requires a path")?.into());
            }
            Some("--window-size") => {
                let mut number = || {
                    args.next()
                        .and_then(|v| v.to_str().and_then(|s| s.parse::<u32>().ok()))
                        .filter(|n| (320..=4096).contains(n))
                        .ok_or("Window dimensions must be 320..4096")
                };
                config.size = Some((number()? as f32, number()? as f32));
            }
            Some("--help") => {
                println!(
                    "Lightwell: [--open JPEG]... [--catalog CATALOG] [--data-root DIRECTORY] [--evidence-dir NEW_DIRECTORY] [--window-size WIDTH HEIGHT]\nEvidence mode imports each --open in order into an isolated catalog, captures a frame after each and exits."
                );
                std::process::exit(0)
            }
            _ => return Err("Unknown argument; use --help".into()),
        }
    }
    if config.files.len() > 16 {
        return Err("At most 16 evidence requests are supported per run".into());
    }
    if config.files.len() > 1 && config.evidence.is_none() {
        return Err("Repeated --open requires --evidence-dir".into());
    }
    if let Some(path) = &config.evidence {
        if path.exists() {
            return Err("Evidence directory must be new to prevent stale evidence".into());
        }
        std::fs::create_dir_all(path)
            .map_err(|e| format!("Cannot create evidence directory: {}", e.kind()))?;
    }
    config.run_id = format!(
        "{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );
    let resolved = paths::Paths::resolve(config.data_root.as_ref());
    // Config and cache are deliberately not created until they have real work.
    if let Some(paths) = &resolved {
        debug_assert!(paths.config != paths.cache);
    }
    let log_dir = config.evidence.clone().or_else(|| {
        config
            .data_root
            .as_ref()
            .and(resolved.as_ref().map(|p| p.logs.clone()))
    });
    if let Some(dir) = log_dir {
        let start = || -> std::io::Result<Diagnostics> {
            std::fs::create_dir_all(&dir)?;
            Diagnostics::start(&dir.join("events.jsonl"))
        };
        match start() {
            Ok(log) => {
                log.panic_hook(config.run_id.clone());
                config.diagnostics = Some(log);
            }
            Err(error) if config.evidence.is_some() => {
                return Err(format!(
                    "diagnostics: cannot initialize log: {}",
                    error.kind()
                ));
            }
            Err(error) => eprintln!(
                "diagnostics: logging unavailable: {}; viewing continues",
                error.kind()
            ),
        }
    }
    Ok(config)
}

fn main() {
    let config = arguments().unwrap_or_else(|error| {
        eprintln!("{error}");
        std::process::exit(2)
    });
    let size = config.size.unwrap_or((960., 640.));
    if let Err(error) = editor_app::run(config, size) {
        eprintln!("Could not start Lightwell editor: {error}");
        std::process::exit(1);
    }
}
