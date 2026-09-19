use crate::*;
use std::{
    process::{Child, Stdio},
    time::{Duration, Instant},
};
pub struct Guard(pub Child);
impl Drop for Guard {
    fn drop(&mut self) {
        if !matches!(self.0.try_wait(), Ok(Some(_))) {
            let _ = self.0.kill();
        }
        let _ = self.0.wait();
    }
}
pub fn spawn(root: &Path, bin: &Path, args: &[OsString], log: &Path) -> Result<Guard> {
    let f = fs::File::create(log)?;
    Ok(Guard(
        Command::new(bin)
            .args(args)
            .current_dir(root)
            .stdin(Stdio::null())
            .stdout(f.try_clone()?)
            .stderr(f)
            .spawn()?,
    ))
}
pub fn wait(child: &mut Guard, timeout: Duration) -> Result<std::process::ExitStatus> {
    let start = Instant::now();
    loop {
        if let Some(status) = child.0.try_wait()? {
            return Ok(status);
        }
        ensure(
            start.elapsed() < timeout,
            "Child timed out; killed and reaped",
        )?;
        std::thread::sleep(Duration::from_millis(10));
    }
}
pub fn pixels(path: &Path, orientation: u8, aspect: Option<f64>) -> Result<Value> {
    ensure((1..=8).contains(&orientation), "Orientation must be 1..8")?;
    let img = image::open(path)?.to_rgb8();
    let (w, h) = img.dimensions();
    let colors =
        crate::fixtures::ORDERS[(orientation - 1) as usize].map(|i| crate::fixtures::COLORS[i]);
    let matches = |p: &[u8], c: [u8; 3]| p.iter().zip(c).all(|(a, b)| a.abs_diff(b) <= 8);
    let (mut left, mut top, mut right, mut bottom) = (w, h, 0, 0);
    let mut found = false;
    for y in (0..h).step_by(4) {
        for x in (0..w).step_by(4) {
            if colors.iter().any(|c| matches(&img.get_pixel(x, y).0, *c)) {
                found = true;
                left = left.min(x);
                top = top.min(y);
                right = right.max(x + 4);
                bottom = bottom.max(y + 4);
            }
        }
    }
    ensure(found, "No fixture pixels: blank or wrong render")?;
    let width = right - left;
    let height = bottom - top;
    ensure(
        width as f64 > w as f64 * 0.2 && height as f64 > h as f64 * 0.5,
        "Fixture too small",
    )?;
    let aspect = aspect.unwrap_or(if orientation >= 5 {
        2.0 / 3.0
    } else {
        3.0 / 2.0
    });
    ensure(aspect.is_finite() && aspect > 0.0, "Invalid aspect")?;
    ensure(
        (width as f64 / height as f64 - aspect).abs() < 0.015,
        "Incorrect Fit aspect ratio",
    )?;
    ensure(
        ((left + right) as f64 / 2.0 - w as f64 / 2.0).abs() <= 5.0,
        "Image not centered",
    )?;
    let mut actual = Vec::new();
    for ((fx, fy), color) in [(0.25, 0.25), (0.75, 0.25), (0.25, 0.75), (0.75, 0.75)]
        .into_iter()
        .zip(colors)
    {
        let x = (left as f64 + fx * width as f64).round() as u32;
        let y = (top as f64 + fy * height as f64).round() as u32;
        ensure(x < w && y < h, "Pixel bounds")?;
        let p = img.get_pixel(x, y).0;
        ensure(matches(&p, color), "Wrong orientation/color")?;
        actual.push(p);
    }
    Ok(
        json!({"status":"passed","physical_size":[w,h],"image_bounds":[left,top,right,bottom],"corner_rgb":actual,"tolerance_per_channel":8,"scope":"Fit geometry and sRGB interiors; not monitor calibration or native picker"}),
    )
}
pub fn events(path: &Path) -> Result<Vec<Value>> {
    fs::read_to_string(path)?
        .lines()
        .map(|l| Ok(serde_json::from_str(l)?))
        .collect()
}
pub fn verify(evidence: &Path, scenario: &str, count: usize) -> Result<Value> {
    let app = read_json(&evidence.join("result.json"))?;
    let events = events(&evidence.join("events.jsonl"))?;
    ensure(
        events.first().is_some_and(|e| e["event"] == "startup")
            && events.last().is_some_and(|e| e["event"] == "shutdown"),
        "Missing lifecycle",
    )?;
    ensure(app["status"] == "captured", "Unsuccessful app result")?;
    ensure(
        app["run_id"].as_str().is_some_and(|s| !s.is_empty())
            && events.iter().all(|e| e["run_id"] == app["run_id"]),
        "Wrong log run identity",
    )?;
    let frames = app["frames"].as_array().ok_or("Missing frames")?;
    ensure(frames.len() == count.max(1), "Missing/stale frames")?;
    for (index, frame) in frames.iter().enumerate() {
        let state = &frame["state"];
        let generation = if count == 0 { 0 } else { index + 1 };
        let orientation =
            if scenario.starts_with("large") || (scenario == "alternating" && index % 2 == 1) {
                1
            } else {
                6
            };
        ensure(state["run_id"] == app["run_id"], "Wrong frame run identity")?;
        ensure(
            state["requested_generation"] == generation,
            "Wrong requested generation",
        )?;
        ensure(
            ["backend", "adapter"]
                .iter()
                .all(|k| state["backend"][k].as_str().is_some_and(|s| !s.is_empty())),
            "Missing backend",
        )?;
        ensure(
            frame["capture_provenance"] == "window-renderer-readback",
            "Wrong capture provenance",
        )?;
        let name = Path::new(frame["file"].as_str().ok_or("Missing capture filename")?);
        ensure(
            name.components()
                .all(|c| matches!(c, std::path::Component::Normal(_))),
            "Unsafe capture path",
        )?;
        let path = evidence.join(name);
        if matches!(scenario, "empty" | "invalid") {
            ensure(
                state["phase"]
                    == if scenario == "empty" {
                        "empty"
                    } else {
                        "error"
                    }
                    && state["displayed_generation"] == 0,
                "Wrong empty/error state",
            )?;
            if scenario == "invalid" {
                ensure(state["error_code"] == "invalid-input", "Wrong error")?;
            }
            let img = image::open(&path)?.to_rgb8();
            let colors: std::collections::BTreeSet<_> =
                img.pixels().map(|p| p.0).take(20_000_000).collect();
            ensure(colors.len() > 10, "Blank empty UI")?;
        } else {
            let displayed = if matches!(scenario, "repeated" | "alternating") {
                generation
            } else {
                1
            };
            ensure(
                state["displayed_generation"] == displayed,
                "Stale displayed image",
            )?;
            let dims = match scenario {
                "large24" => [6000, 4000],
                "large60" => [10000, 6000],
                _ => {
                    if orientation == 1 {
                        [480, 320]
                    } else {
                        [320, 480]
                    }
                }
            };
            ensure(
                state["source_dimensions"] == json!(dims),
                "Wrong dimensions",
            )?;
            let failed = scenario == "replacement" && index == 1;
            ensure(
                state["phase"] == if failed { "error" } else { "ready" },
                "Wrong phase",
            )?;
            if failed {
                ensure(
                    state["error_code"] == "invalid-input",
                    "Wrong replacement error",
                )?;
            }
            pixels(
                &path,
                orientation,
                match scenario {
                    "large24" => Some(1.5),
                    "large60" => Some(5.0 / 3.0),
                    _ => None,
                },
            )?;
            ensure(
                events.iter().any(|e| {
                    e["event"] == "render_ready" && e["generation"] == state["displayed_generation"]
                }),
                "Missing upload readiness",
            )?;
        }
    }
    Ok(app)
}
pub fn run(root: &Path, out: &Path, scenario: &str, bin: &Path, timeout: Duration) -> Result {
    ensure(!out.exists(), "Smoke output must be new")?;
    let fixture = root.join("fixtures/s0/orientation-6.jpg");
    let sources = match scenario {
        "empty" => vec![],
        "load" => vec![fixture],
        "replacement" => vec![fixture, root.join("fixtures/s0/invalid.jpg")],
        "invalid" => vec![root.join("fixtures/s0/invalid.jpg")],
        "repeated" => vec![fixture; 8],
        "alternating" => (0..4)
            .flat_map(|_| [fixture.clone(), root.join("fixtures/s0/orientation-1.jpg")])
            .collect(),
        "large24" => vec![root.join("fixtures/generated/24mp.jpg")],
        "large60" => vec![root.join("fixtures/generated/60mp.jpg")],
        _ => return Err("Unknown smoke scenario".into()),
    };
    fs::create_dir_all(out)?;
    let evidence = out.join("app");
    let mut args = vec!["--evidence-dir".into(), evidence.clone().into_os_string()];
    for p in &sources {
        args.extend(["--open".into(), p.as_os_str().into()]);
    }
    let command = std::iter::once(bin.as_os_str())
        .chain(args.iter().map(OsString::as_os_str))
        .map(|s| s.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let mut result = json!({"scenario":scenario,"status":"failed","command":command,"platform":format!("{}-{}",std::env::consts::OS,std::env::consts::ARCH)});
    let check = (|| -> Result {
        let mut hashes = serde_json::Map::new();
        for p in &sources {
            hashes.insert(
                p.file_name().unwrap().to_string_lossy().into_owned(),
                json!(hash(p)?),
            );
        }
        result["fixture_hashes"] = json!(hashes);
        result["binary_sha256"] = json!(hash(bin)?);
        result["lockfile_sha256"] = json!(hash(&root.join("Cargo.lock"))?);
        let mut child = spawn(root, bin, &args, &out.join("subprocess.log"))?;
        let status = wait(&mut child, timeout)?;
        result["exit_code"] = json!(status.code());
        ensure(status.success(), format!("Application exit {status}"))?;
        let app = verify(&evidence, scenario, sources.len())?;
        for p in &sources {
            ensure(
                json!(hash(p)?)
                    == result["fixture_hashes"][p.file_name().unwrap().to_string_lossy().as_ref()],
                "Source changed",
            )?;
        }
        result["backend"] =
            app["frames"].as_array().unwrap().last().unwrap()["state"]["backend"].clone();
        Ok(())
    })();
    match &check {
        Ok(()) => result["status"] = json!("passed"),
        Err(e) => result["error"] = json!(e.to_string()),
    };
    write_json(&out.join("result.json"), &result)?;
    fs::write(
        out.join("reproduce.md"),
        format!(
            "# Smoke run\n\nScenario: {scenario}. Status: {}.\n\nArgument array:\n\n```json\n{}\n```\n\nActual renderer readback; native dialog/focus verified separately. Synthetic fixtures only.\n",
            result["status"],
            serde_json::to_string_pretty(&command)?
        ),
    )?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    check
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_binary_retains_failure() {
        let tmp = tempfile::tempdir().unwrap();
        let out = tmp.path().join("evidence ü space");
        assert!(
            run(
                &root().unwrap(),
                &out,
                "load",
                &tmp.path().join("absent"),
                Duration::from_millis(100)
            )
            .is_err()
        );
        assert_eq!(
            read_json(&out.join("result.json")).unwrap()["status"],
            "failed"
        );
        assert!(out.join("reproduce.md").is_file());
        assert!(!out.join("app/frame-1.png").exists());
    }
    #[test]
    fn missing_evidence_and_stale_generation_fail() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(verify(tmp.path(), "load", 1).is_err());
        fs::write(tmp.path().join("events.jsonl"),"{\"event\":\"startup\",\"run_id\":\"current\"}\n{\"event\":\"shutdown\",\"run_id\":\"current\"}\n").unwrap();
        let mut app = json!({"status":"captured","run_id":"current","frames":[{"state":{"run_id":"old","requested_generation":1}}]});
        write_json(&tmp.path().join("result.json"), &app).unwrap();
        assert!(
            verify(tmp.path(), "load", 1)
                .unwrap_err()
                .to_string()
                .contains("run identity")
        );
        app["frames"][0]["state"]["run_id"] = json!("current");
        app["frames"][0]["state"]["requested_generation"] = json!(0);
        write_json(&tmp.path().join("result.json"), &app).unwrap();
        assert!(
            verify(tmp.path(), "load", 1)
                .unwrap_err()
                .to_string()
                .contains("generation")
        );
    }
    #[test]
    fn blank_frame_fails() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("blank.png");
        image::RgbImage::new(960, 640).save(&p).unwrap();
        assert!(
            pixels(&p, 6, None)
                .unwrap_err()
                .to_string()
                .contains("blank")
        );
    }
    #[test]
    #[ignore]
    fn sleeping_child() {
        std::thread::sleep(Duration::from_secs(30));
    }
    #[test]
    fn timeout_reaps_child() {
        let tmp = tempfile::tempdir().unwrap();
        let mut child = spawn(
            &root().unwrap(),
            &std::env::current_exe().unwrap(),
            &[
                "--ignored".into(),
                "--exact".into(),
                "smoke::tests::sleeping_child".into(),
            ],
            &tmp.path().join("child.log"),
        )
        .unwrap();
        assert!(
            wait(&mut child, Duration::from_millis(50))
                .unwrap_err()
                .to_string()
                .contains("timed out")
        );
        child.0.kill().unwrap();
        assert!(child.0.wait().is_ok());
    }
}
