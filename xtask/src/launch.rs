//! Non-interactive test launches must not activate the owner's desktop.
use crate::*;

/// Keep the temporary bundle alive until its child has exited and been reaped.
pub struct Background {
    pub executable: PathBuf,
    #[cfg(target_os = "macos")]
    _bundle: tempfile::TempDir,
}

pub const MODE: &str = if cfg!(target_os = "macos") {
    "macos-background-bundle"
} else {
    "direct"
};

impl Background {
    pub fn new(binary: &Path) -> Result<Self> {
        #[cfg(target_os = "macos")]
        {
            // Iced's winit runner activates ordinary unbundled executables. A background-only
            // bundle prevents activation before the event loop starts, while renderer readbacks
            // still use the native GPU. Do not use a symlink: Cocoa resolves it to the original
            // executable and loses the bundle identity. Never modify the built/packaged app.
            let bundle = tempfile::Builder::new()
                .prefix("lightwell-test-")
                .tempdir()?;
            let contents = bundle.path().join("Lightwell Test.app/Contents");
            fs::create_dir_all(contents.join("MacOS"))?;
            let executable = contents.join("MacOS/lightwell-test");
            fs::copy(binary, &executable)?;
            fs::write(
                contents.join("Info.plist"),
                r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><dict>
<key>CFBundleIdentifier</key><string>org.lightwell.background-test</string>
<key>CFBundleName</key><string>Lightwell Test</string>
<key>CFBundleExecutable</key><string>lightwell-test</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>LSBackgroundOnly</key><true/>
<key>NSHighResolutionCapable</key><true/>
</dict></plist>
"#,
            )?;
            Ok(Self {
                executable,
                _bundle: bundle,
            })
        }
        #[cfg(not(target_os = "macos"))]
        {
            Ok(Self {
                executable: binary.into(),
            })
        }
    }
}
