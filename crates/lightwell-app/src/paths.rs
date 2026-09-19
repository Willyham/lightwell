use std::path::PathBuf;
#[derive(Clone, Debug)]
pub struct Paths {
    pub config: PathBuf,
    pub cache: PathBuf,
    pub logs: PathBuf,
}
impl Paths {
    pub fn resolve(root: Option<&PathBuf>) -> Option<Self> {
        if let Some(root) = root {
            return Some(Self {
                config: root.join("config"),
                cache: root.join("cache"),
                logs: root.join("logs"),
            });
        }
        let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
            .map(PathBuf::from)?;
        if cfg!(target_os = "macos") {
            Some(Self {
                config: home.join("Library/Application Support/Lightwell"),
                cache: home.join("Library/Caches/Lightwell"),
                logs: home.join("Library/Logs/Lightwell"),
            })
        } else if cfg!(windows) {
            let local = PathBuf::from(std::env::var_os("LOCALAPPDATA")?);
            let roaming = PathBuf::from(std::env::var_os("APPDATA")?);
            Some(Self {
                config: roaming.join("Lightwell"),
                cache: local.join("Lightwell/Cache"),
                logs: local.join("Lightwell/Logs"),
            })
        } else {
            let base = |key, fallback| {
                std::env::var_os(key)
                    .map(PathBuf::from)
                    .filter(|p| p.is_absolute())
                    .unwrap_or_else(|| home.join(fallback))
            };
            Some(Self {
                config: base("XDG_CONFIG_HOME", ".config").join("lightwell"),
                cache: base("XDG_CACHE_HOME", ".cache").join("lightwell"),
                logs: base("XDG_STATE_HOME", ".local/state").join("lightwell/logs"),
            })
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn override_redirects_every_path_without_creating_it() {
        let root = std::env::temp_dir().join("Lightwell isolated ü paths");
        let paths = Paths::resolve(Some(&root)).unwrap();
        assert_eq!(paths.config, root.join("config"));
        assert_eq!(paths.cache, root.join("cache"));
        assert_eq!(paths.logs, root.join("logs"));
    }
}
