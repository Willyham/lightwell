# S0 UI and JPEG probes

Status: isolated experimental workspace, separate from the accepted maintained viewer. Iced is the selected application framework. These probes provide bounded rendering and decoder evidence, not editor features or native cross-platform acceptance.

## Reproduce

From the repository root, with Rust 1.94.0 and platform development tools installed:

```sh
cargo xtask check-repository
cargo xtask fixtures
cargo test --manifest-path probes/s0/Cargo.toml --locked --lib
cargo fmt --manifest-path probes/s0/Cargo.toml -- --check
cargo clippy --manifest-path probes/s0/Cargo.toml --locked --all-targets --features iced-ui,egui-ui -- -D warnings
cargo build --manifest-path probes/s0/Cargo.toml --locked --release --features iced-ui,egui-ui
cargo xtask probe --candidate iced --output artifacts/iced-new-run
cargo xtask check-capture --image artifacts/iced-new-run/window.png
cargo xtask probe --candidate egui --output artifacts/egui-new-run
cargo xtask check-capture --image artifacts/egui-new-run/window.png
```

Use a fresh output directory for every capture. The probe runner kills a non-completing process at 30 seconds and retains logs/results. Rendering uses a native window and renderer readback, not an offscreen substitute or OS-compositor screenshot. It does not prove file-dialog, focus, resizing, or package behavior. See the [fixture setup](../../fixtures/README.md) for Rust verification commands. Cargo needs network on initial dependency fetch; `--offline` was used for the recorded checks after fetch. To use the pinned compiler when another default is installed, run Cargo from `probes/s0` or use `cargo +1.94.0`.

An interactive trial accepts an optional JPEG path and optional capture PNG path. Without a capture path it remains open; without arguments it shows the empty state. These binaries live under `probes/s0/target/release/`. They are experiments, not user installation commands or the later command API. On Windows the binary suffix is `.exe`; native probe evidence is from macOS; Windows execution remains unverified.

## Configuration and limits

The [trial manifest](../../probes/s0/Cargo.toml) pins Iced 0.14.0, eframe 0.33.3, image 0.25.9, moxcms 0.7.11, rfd 0.15.4 and serde_json 1.0.149. Both UI paths resolve wgpu 27.0.1 with Metal on the measured host. The open service permits JPEG only; PNG is used for evidence.

Input limits are 128 MiB source, 16384 pixels per side, 64 million pixels and 512 MiB decoder allocation. These are not total RSS bounds. One worker handles active decode and a replaceable pending path; stale generations never replace a newer request. Active decode is not interrupted mid-call.

The current profile recognizer accepts RGB matrix/TRC input/display/colorspace profiles with XYZ PCS, standard sRGB D50 colorants/white point within 0.0005, and transfer values within 0.0005 linear-light units at every 8-bit code. LUT transforms, other gamuts, conflicting CICP, unsupported classes, malformed profiles and non-RGB tagged inputs are rejected. ICC parsing is bounded to 1 MiB, 4096 TRC entries and 4096 CLUT bytes. Untagged greyscale is accepted. Names, manufacturers and timestamps do not establish color support.

Unit checks cover independently serialized standard-sRGB profiles, benign header changes, altered gamma, Adobe RGB, malformed input, all EXIF orientations and source preservation. This conservative recognition policy is not full ICC conversion or professional display calibration.

## Verified probe evidence

Measurements recorded 2026-09-19 on macOS 26.5.2 arm64, M4 Pro with 14 CPU/20 GPU cores and 48 GB unified memory. Captures are 1920×1280 physical pixels for a 960×640 logical window at scale 2 on hardware Metal. Display profile and storage conditions were not established.

- Sixteen synthetic fixtures pass hash, metadata/dimension and independent corner-orientation checks; 24/60 MP workloads are generated in ignored storage.
- Both optimized UIs built and captured orientation-6 content correctly: aspect, centering, substantial photo area and quadrant colors within eight levels/channel. Both frames were visually inspected.
- Iced capture/shutdown took 1.148 s and egui 0.721 s in single whole-process observations including PNG output/shutdown. These are not comparative startup benchmarks or percentile claims.
- Native Iced interaction verified empty state, Cmd+O, picker load, resize/Fit and cancel retention. This is separate from automated renderer readback and maintained-app acceptance.

Ignored evidence in `artifacts/s0-probes/iced-native/` and `artifacts/s0-probes/egui-native-fixed/` contains result, process log, window PNG and pixel checks. `captured` reports capture/process success; separate pixel results establish image validation. Source hashes remain unchanged.

### JPEG measurements

Optimized pure decode including file read, orientation and RGBA conversion; 30 fresh processes per size, sequential, warm filesystem cache, synthetic repeated-color/detail inputs. No cache flush, photographic entropy representativeness or request-to-present timing is claimed.

| Input | Median | p95 (nearest rank) | RGBA allocation |
| --- | --- | --- | --- |
| 24 MP / 6000×4000 | 33.80 ms | 35.06 ms | 96,000,000 bytes |
| 60 MP / 10000×6000 | 86.68 ms | 91.81 ms | 240,000,000 bytes |

A separate `/usr/bin/time -l probes/s0/target/release/decode fixtures/generated/60mp.jpg` run measured maximum RSS 426,147,840 bytes (406.4 MiB), peak footprint 425,329,288 bytes. This is CPU-only decode, not viewer/GPU memory. Logs are `60mp-memory.json` and `60mp-memory.txt`. Original source bytes were only read.

## Evidence limits

Renderer captures and decoder timings identify their measured configurations; profile-recognition unit coverage is not a new native color measurement. No Windows/Linux probe desktop run, calibrated display proof or comparative long-run framework benchmark is claimed. Current maintained-app verification is in [hardening results](s0-hardening-results.md), [CI results](ci-results.md) and the [platform matrix](platforms.md).
