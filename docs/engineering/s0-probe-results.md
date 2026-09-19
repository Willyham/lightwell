# S0 scaffold and probe results — 2026-09-19

Status: **experimental, not the maintained S0 application**. Repository/fixture tasks are complete; JPEG probe evidence is recorded. UI trials compile and render on the M4, with native interaction checks still outstanding. No final stack has been selected. Subsequent owner answer selected GPL-3.0-or-later; license application and dependency audit remain pending.

## Reproduce

From the repository root, with Rust 1.94.0 and platform development tools installed:

```sh
python3 tools/check_repository.py
python3 tools/check_fixtures.py
cargo test --manifest-path probes/s0/Cargo.toml --locked --lib
cargo fmt --manifest-path probes/s0/Cargo.toml -- --check
cargo clippy --manifest-path probes/s0/Cargo.toml --locked --all-targets --features iced-ui,egui-ui -- -D warnings
cargo build --manifest-path probes/s0/Cargo.toml --locked --release --features iced-ui,egui-ui
python3 tools/run_ui_probe.py iced --output artifacts/iced-new-run
python3 tools/check_probe_capture.py artifacts/iced-new-run/window.png
python3 tools/run_ui_probe.py egui --output artifacts/egui-new-run
python3 tools/check_probe_capture.py artifacts/egui-new-run/window.png
```

Use a fresh output directory for every capture. The probe runner kills a non-completing process at 30 seconds and retains logs/results. Rendering uses a native window and renderer readback, not an offscreen substitute or OS-compositor screenshot. It does not prove file-dialog, focus, resizing, or package behavior. Pillow 12.2.0 is needed for fixture/pixel checks; see the [fixture setup](../../fixtures/README.md). Cargo needs network on initial dependency fetch; `--offline` was used for the recorded checks after fetch. To use the pinned compiler when another default is installed, run Cargo from `probes/s0` or use `cargo +1.94.0`.

An interactive trial accepts an optional JPEG path and optional capture PNG path. Without a capture path it remains open; without arguments it shows the empty state. These binaries live under `probes/s0/target/release/`. They are experiments, not user installation commands or the later command API. On Windows the binary suffix is `.exe`; the current Python UI runner is host-tested on macOS only and needs that adaptation before a Windows claim.

## Configuration and limits

The [trial manifest](../../probes/s0/Cargo.toml) and lockfile pin Iced 0.14.0, eframe 0.33.3, image 0.25.9, rfd 0.15.4 and serde_json 1.0.149. Both UI paths resolve wgpu 27.0.1 with Metal on this host. Iced uses `image-without-codecs`, wgpu, tokio, sysinfo, X11 and Wayland. Eframe uses wgpu/default fonts/X11/Wayland. The image decoder itself uses the pure-Rust zune-jpeg 0.5.15 path. Native JPEG or RAW libraries are not required. PNG is enabled for evidence encoding; eframe's clipboard dependency additionally enables TIFF in the combined build. The open service explicitly restricts input to JPEG, irrespective of compiled codec features.

The primary crates report MIT or MIT/Apache-2.0. These are registry metadata observations, not completion of the configured dependency/font/native-license audit in TASK-041. Rust/macOS SDK/linker and Metal/AppKit frameworks were present. Windows requires its native linker/SDK and Linux requires native window/graphics libraries and an XDG file-dialog portal; exact supported distributions/runtime floors remain product/stack work. No installation changes or VM setup were performed.

Trial input accepts 8-bit 1/3-component JPEG, orientations 1–8, untagged as sRGB and the exact generated embedded sRGB profile. Other ICC bytes and CMYK are rejected explicitly. The profile allowlist is deliberately narrow and not suitable as a claim of general tagged-sRGB support. JPEG SOI/EOI and segment bounds are checked before decode; not every possible corrupt entropy stream has been tested. Limits: 128 MiB source, 16384 pixels per side, 64 million pixels, 512 MiB decoder allocation limit. The latter is not a total RSS bound: RGB/RGBA conversion and UI/GPU copies add memory.

A worker handles one active decode and one replaceable pending path; generation filtering prevents stale results replacing newer requests. The active decoder is not interrupted mid-call. The UI retains its last texture on error and polls only while a decode is active. Tests cover rapid valid/invalid/latest requests. No catalog, persistence, geometry, export, IPC/MCP or production diagnostics subsystem was added.

The API choices were checked against the primary [Iced screenshot contract](https://docs.rs/iced/0.14.0/iced/window/fn.screenshot.html), [eframe 0.33.3 documentation](https://docs.rs/eframe/0.33.3/eframe/), and [image decoder interface](https://docs.rs/image/0.25.9/image/trait.ImageDecoder.html), then against the locked local crate sources. Iced documents RGBA screenshot bytes as sRGB. Pixel evidence below checks the complete input/texture/capture path for the synthetic subset, not monitor calibration or general ICC display management.

## Evidence

Host: macOS 26.5.2 arm64, M4 Pro, 14 CPU / 20 GPU cores, 48 GB unified memory. Captures are 1920×1280 physical pixels for a 960×640 logical window, scale 2, hardware Metal adapter `Apple M4 Pro`. Display profile and storage conditions were not established, so no professional color or cold-storage-performance claim is made.

- Fixture checker: 16 small fixtures, exact regeneration with installed Pillow 12.2.0 / JPEG API 6.2, SHA-256 validation, metadata/dimensions and all eight independent corner-orientation expectations. Large 24/60 MP images are generated in ignored storage. Checked-in small corpus: approximately 468 KiB.
- Rust tests: supported subset, all orientations with per-channel tolerance 5, invalid/truncated/oversized/CMYK/profile/missing errors, exact source preservation, and newest-request behavior.
- Repository checks: both schemas/DAGs, task status prerequisites, derived waves, globally unique IDs, local file links and external completion gates. Negative checks rejected cycles, unknown keys, premature completion and stale waves. Markdown anchors are not yet validated.
- Both optimized UIs built on macOS arm64. Iced's capture/shutdown took 1.148 seconds; corrected egui took 0.721 seconds. These are single whole-process observations including PNG write and shutdown, **not** comparable startup benchmarks or p95 measurements.
- Both captures passed independent orientation-6 color/geometry checks: 2:3 image aspect, horizontal centering, substantial photo area, and all four colored quadrants within 8 levels/channel. Actual interiors were blue `(40,70,220)`, red `(220,34,45)`, gold `(234,195,30)`, green `(35,189,65)`. Both were also visually inspected for clipping/layout/detail. This verifies the captured synthetic photo; it does not cover all visual states.

Local ignored artifacts: `artifacts/s0-probes/iced-native/` and `egui-native-fixed/` contain `result.json`, `process.log`, `window.png`, `window.pixels.json`. Result status `captured` means successful capture and process exit; the separate `.pixels.json` records pixel validation. The runner verifies unchanged source SHA-256. `decode-benchmark.json` contains all 30 per-size samples. Evidence is retained locally, not committed or uploaded.

### JPEG measurements

Optimized pure decode including file read, orientation and RGBA conversion; 30 fresh processes per size, sequential, warm filesystem cache, synthetic repeated-color/detail inputs. No cache flush, photographic entropy representativeness or request-to-present timing is claimed.

| Input | Median | p95 (nearest rank) | RGBA allocation |
| --- | --- | --- | --- |
| 24 MP / 6000×4000 | 33.80 ms | 35.06 ms | 96,000,000 bytes |
| 60 MP / 10000×6000 | 86.68 ms | 91.81 ms | 240,000,000 bytes |

A separate `/usr/bin/time -l probes/s0/target/release/decode fixtures/generated/60mp.jpg` run measured maximum RSS 426,147,840 bytes (406.4 MiB), peak footprint 425,329,288 bytes. This is CPU-only decode, not viewer/GPU memory. Logs are `60mp-memory.json` and `60mp-memory.txt`. Original source bytes were only read.

### Failures retained and remaining checks

The first sandboxed GUI launch could not reach macOS desktop services and timed out. Outside that sandbox both trials rendered on Metal. An initial egui screenshot callback nested context access under an input lock and deadlocked after writing the PNG; events are now copied out before handling captures, and the corrected run exits successfully. Both failures remain in local evidence and do not count as passing native runs.

Computer-use lookup could not address the bare executable, so an ignored local test bundle was assembled. Automatic approval review rejected opening that bundle as an unrecognized local software source requiring action-time confirmation. Owner approval is pending. Native picker, cancellation, shortcut/focus, resize, empty/error screen review and extended startup/idle/memory comparisons remain unverified. No workaround was used after that rejection; the already-running interactive process was stopped.

Windows/Linux compilation and desktop sessions are not verified. No final application packages, CI runs, production smoke protocol or S0 completion are claimed. TASK-003 remains in progress. TASK-036 is complete as a bounded technical probe with the above color/profile limitations; maintained input policy is still a product/stack decision.

## Recommendation and next gate

Keep Iced as the leading candidate: this trial supplies an adequate photo surface, straightforward renderer capture and a more deliberate initial shell. Eframe remains viable; no decisive performance comparison has been made. Finish the native interaction checks before selecting the stack in TASK-005. Product TASK-023–025 must be answered before TASK-035 can complete. The maintained workspace TASK-006 follows those prerequisites; S0 acceptance remains TASK-063 with all three platform launch/load checks.

Active plan validation: product decisions 13 tasks / 1 wave; implementation 51 tasks / 23 waves. Both graphs validate without renumbering IDs or modifying archived history. At this checkpoint TASK-003 is the active technical follow-up; maintained workspace work has no runnable path until its documented product/stack prerequisites complete.

## Follow-up after owner decisions

Owner approval now covers the local app build/run scope; it resolves the earlier automatic-review permission block. The next computer-use launch attempt reported **Mac locked; automatic unlock unavailable**. Unlock was requested. Native picker/focus/resize and the final UI selection remain pending; no replacement interaction or successful new screenshot is claimed.

Product TASK-023/024/025 and implementation TASK-035 are now complete. See the [platform matrix](platforms.md) for delegated provisional OS baselines and explicit outstanding test routes. No further product approval is needed to choose those baselines.

The JPEG probe now directly pins moxcms 0.7.11 (already present transitively via image; BSD-3-Clause) for bounded ICC parsing. This **supersedes the earlier exact-profile allowlist**. Accepted embedded profiles are RGB matrix/TRC input/display/colorspace profiles with XYZ PCS, standard sRGB D50 colorants/white point within 0.0005, and standard sRGB transfer values within 0.0005 linear-light units at every possible 8-bit source code. LUT transforms, other gamuts, conflicting CICP primaries/transfer, unsupported classes, malformed profiles and non-RGB tagged inputs are rejected. ICC parsing limits are 1 MiB profile, 4096 TRC entries and 4096 CLUT bytes. Untagged greyscale remains accepted. Header labels, manufacturers and timestamps do not establish color support. Broader LUT/greyscale ICC conversion remains excluded.

Tests pass against the original Little CMS/Pillow sRGB fixture and an independently serialized moxcms sRGB profile, including benign header changes; Adobe RGB, altered gamma and malformed profiles are rejected. All five Rust tests and strict Clippy checks pass. This is a conservative numeric-recognition policy with explicit tolerances; it does not claim all profiles named sRGB are supported or professional display accuracy. Earlier renderer captures and timing measurements remain evidence from the initial probe configuration; this follow-up is parser/unit evidence, not a new native verification run.
