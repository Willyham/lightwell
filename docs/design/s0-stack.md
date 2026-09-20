# S0 stack and boundaries

Status: implemented Rust/Iced viewer; S0 is accepted. Native M4 behavior is verified. Minimum-OS compatibility, native Windows/Linux sessions, full accessibility and broad ICC/display calibration remain unverified.

## Selected stack

Rust 1.94.0, Iced 0.14.0 with wgpu 27.0.1, image 0.25.9 for JPEG input and PNG evidence, moxcms 0.7.11 for conservative standard-sRGB profile recognition, rfd 0.15.4 for native/portal dialogs, and serde_json 1.0.149 for evidence. Exact transitive versions are in Cargo.lock. Project code is GPL-3.0-or-later; manual dependency/native/asset review remains deferred.

Iced supplies explicit state updates, reactive rendering, a simple image surface and window-renderer capture. The isolated [UI probes](../engineering/s0-probe-results.md) verify both Iced and egui rendering on M4 Metal; they do not establish a comparative performance winner. Custom-control accessibility is limited and requires further work.

## Workspace

- `crates/lightwell-core`: read-only JPEG decode/profile/resource rules, typed errors and bounded open requests.
- `crates/lightwell-app`: Iced window, native adapters, diagnostics and renderer evidence.
- `xtask`: Rust development/check/build/package and verification commands.
- `probes/s0`: isolated UI/decoder comparison workspace, not the maintained application.

The maintained viewer opens supported JPEGs off the UI thread, applies EXIF orientation once, rejects stale generations and displays a bounded preview at Fit. Source bytes stay unchanged; a failed replacement retains the visible photo. There are no catalog/editor/IPC/plugin crates or implemented editing features.

## Profile and resource contract

Standard-sRGB recognition uses bounded numeric matrix/TRC tests, not fixture bytes or profile names. Limits are 128 MiB source, 64 MP, 16384 pixels per side and a 512 MiB decoder-allocation bound. Preview uploads are bounded to 4096 pixels per side and resized on the worker. Decoder limits are not total process/GPU memory limits.

Allow one active and one latest pending decode. Discard stale completions, never join the decoder on the GUI thread, and wait for explicit GPU allocation before reporting rendered readiness. Source-resolution zoom is planned for M1; S0 is Fit-only.

## Evidence and development

Opt-in `--open PATH`, `--evidence-dir NEW_DIRECTORY`, `--window-size WIDTH HEIGHT` and bounded deadlines exercise the same loader as native Open. Records distinguish requested/displayed generations, source/preview dimensions, renderer backend and physical scale. Background work saves state, frames, events, results and reproduction arguments.

Ordinary viewing creates no catalog or source-adjacent files and does not poll while idle. An explicit data root isolates diagnostics. Failures preserve usable viewing and report evidence-write problems rather than claiming success.

The [command reference](../engineering/scaffold-commands.md), [hardening evidence](../engineering/s0-hardening-results.md), [CI results](../engineering/ci-results.md) and [platform matrix](../engineering/platforms.md) distinguish verified behavior from outstanding checks. Packages are unsigned development artifacts, not distribution or license-audit approval.
