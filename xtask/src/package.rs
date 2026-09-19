use crate::*;
use std::io::Write;
pub fn inventory(root: &Path, out: &Path) -> Result {
    let target = host(root)?;
    let data: Value = serde_json::from_str(&output(
        root,
        "cargo",
        &[
            "metadata",
            "--locked",
            "--format-version",
            "1",
            "--filter-platform",
            &target,
        ],
    )?)?;
    let active: std::collections::BTreeSet<_> = data["resolve"]["nodes"]
        .as_array()
        .ok_or("Missing resolve nodes")?
        .iter()
        .map(|n| n["id"].clone().to_string())
        .collect();
    let mut packages = Vec::new();
    fs::create_dir_all(out.join("licenses"))?;
    for p in data["packages"].as_array().ok_or("Missing packages")? {
        if !active.contains(&p["id"].to_string()) {
            continue;
        }
        packages.push(json!({"name":p["name"],"version":p["version"],"license":p["license"]}));
        let dir = Path::new(p["manifest_path"].as_str().ok_or("Missing manifest")?)
            .parent()
            .ok_or("Manifest parent")?;
        let dest = out.join("licenses").join(format!(
            "{}-{}",
            p["name"].as_str().unwrap(),
            p["version"].as_str().unwrap()
        ));
        for f in fs::read_dir(dir)? {
            let f = f?.path();
            let name = f.file_name().unwrap().to_string_lossy().to_lowercase();
            if f.is_file()
                && ["license", "copying", "notice"]
                    .iter()
                    .any(|p| name.starts_with(p))
            {
                fs::create_dir_all(&dest)?;
                fs::copy(&f, dest.join(f.file_name().unwrap()))?;
            }
        }
    }
    write_json(
        &out.join("dependencies.json"),
        &json!({"target":target,"packages":packages,"review_status":"Inventory only; manual license review deferred"}),
    )?;
    println!("Inventory: {} configured packages", packages.len());
    Ok(())
}
fn archive(directory: &Path, destination: &Path) -> Result {
    if destination.extension().is_some_and(|e| e == "zip") {
        let mut zip = zip::ZipWriter::new(fs::File::create(destination)?);
        for path in files(directory)? {
            let relative = path
                .strip_prefix(directory.parent().unwrap())?
                .to_str()
                .ok_or("Archive path encoding")?
                .replace('\\', "/");
            let mode = if path.file_name().is_some_and(|s| s == "lightwell") {
                0o755
            } else {
                0o644
            };
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated)
                .unix_permissions(mode);
            zip.start_file(relative, options)?;
            std::io::copy(&mut fs::File::open(path)?, &mut zip)?;
        }
        zip.finish()?.sync_all()?;
    } else {
        let gz = flate2::write::GzEncoder::new(
            fs::File::create(destination)?,
            flate2::Compression::default(),
        );
        let mut tar = tar::Builder::new(gz);
        tar.append_dir_all("Lightwell", directory)?;
        tar.into_inner()?.finish()?.sync_all()?;
    }
    Ok(())
}
pub fn package(root: &Path, out: &Path) -> Result {
    ensure(!out.exists(), "Package output must be new")?;
    cargo(root, "build", true)?;
    fs::create_dir_all(out)?;
    let target = out.join("Lightwell");
    fs::create_dir(&target)?;
    let binary = binary(root)?;
    if cfg!(target_os = "macos") {
        let app = target.join("Lightwell.app/Contents");
        fs::create_dir_all(app.join("MacOS"))?;
        fs::copy(&binary, app.join("MacOS/lightwell"))?;
        fs::write(
            app.join("Info.plist"),
            r#"<?xml version="1.0" encoding="UTF-8"?><!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd"><plist version="1.0"><dict><key>CFBundleIdentifier</key><string>org.lightwell.app</string><key>CFBundleName</key><string>Lightwell</string><key>CFBundleExecutable</key><string>lightwell</string><key>CFBundlePackageType</key><string>APPL</string><key>CFBundleShortVersionString</key><string>0.0.0</string><key>LSMinimumSystemVersion</key><string>14.0</string><key>NSHighResolutionCapable</key><true/></dict></plist>"#,
        )?;
    } else {
        fs::copy(&binary, target.join(binary.file_name().unwrap()))?;
    }
    write_json(
        &target.join("build.json"),
        &json!({"revision":output(root,"git",&["rev-parse","HEAD"])?.trim(),"working_tree_dirty":!output(root,"git",&["status","--porcelain"])?.trim().is_empty(),"target":host(root)?,"profile":"release","binary_sha256":hash(&binary)?,"lock_sha256":hash(&root.join("Cargo.lock"))?}),
    )?;
    fs::copy(root.join("LICENSE"), target.join("LICENSE"))?;
    inventory(root, &target.join("notices"))?;
    fs::write(
        target.join("README.txt"),
        "Unsigned development artifact. Manual license/native Windows/Linux reviews are deferred. See docs/engineering/platforms.md for runtime prerequisites.\n",
    )?;
    let archive_path = out.join(if cfg!(target_os = "linux") {
        "lightwell-development.tar.gz"
    } else {
        "lightwell-development.zip"
    });
    archive(&target, &archive_path)?;
    writeln!(
        fs::File::create(out.join("checksums.txt"))?,
        "{}  {}",
        hash(&archive_path)?,
        archive_path.file_name().unwrap().to_string_lossy()
    )?;
    println!("{}", archive_path.display());
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn archives_preserve_payload_and_zip_executable() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("Lightwell");
        fs::create_dir(&dir).unwrap();
        fs::write(dir.join("lightwell"), b"test executable").unwrap();
        let dest = tmp.path().join("test.zip");
        archive(&dir, &dest).unwrap();
        let mut zip = zip::ZipArchive::new(fs::File::open(dest).unwrap()).unwrap();
        let mut file = zip.by_name("Lightwell/lightwell").unwrap();
        assert_eq!(file.unix_mode().unwrap() & 0o777, 0o755);
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, b"test executable");
        let dest = tmp.path().join("test.tar.gz");
        archive(&dir, &dest).unwrap();
        let mut tar =
            tar::Archive::new(flate2::read::GzDecoder::new(fs::File::open(dest).unwrap()));
        let mut found = false;
        for entry in tar.entries().unwrap() {
            let mut entry = entry.unwrap();
            if entry.path().unwrap() == Path::new("Lightwell/lightwell") {
                let mut bytes = Vec::new();
                entry.read_to_end(&mut bytes).unwrap();
                assert_eq!(bytes, b"test executable");
                found = true;
            }
        }
        assert!(found);
    }
}
