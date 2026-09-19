# S0 stack decision and maintained scaffold

Status: selected for v0 S0 under the accepted product decisions. No cross-platform verification or S0 completion is implied.

## Decision and evidence

Select Rust 1.94.0, Iced 0.14.0 with wgpu 27.0.1, image 0.25.9 (JPEG input and PNG evidence), moxcms 0.7.11 for conservative sRGB profile recognition, rfd 0.15.4 for native/portal dialogs, and serde_json 1.0.149 for internal evidence. Exact transitive versions stay in Cargo.lock. Select GPL-3.0-or-later for project code as instructed by the owner; configured license/advisory audit remains TASK-041.

The [probe report](../engineering/s0-probe-results.md) records both Iced/egui builds and M4 Metal captures. The resumed native Iced trial displayed the empty state, opened the native picker with Cmd+O, loaded orientation-6 correctly, recomputed Fit when the native zoom-window action enlarged the window, and retained the image/status after cancelling the picker. OS-window screenshots were inspected in the computer-use session; they are separate from the retained renderer captures. The initial corner-drag automation returned noWindowsAvailable; native zoom supplied the actual resize check. Later automated path entry/selection was unreliable, so native failed-replacement verification remains in the maintained application's acceptance checks. This is not a reason to change UI frameworks.

Prefer Iced's explicit updates, reactive rendering, simple image surface and documented window-renderer capture route. Eframe is viable but provides no decisive advantage in the bounded comparison; its repaired capture trial was successful. Single-run startup/capture figures are not statistically comparable performance evidence. Screen-reader exposure is limited in this probe (native controls/dialogs appear in AX, custom content does not); carry this limitation into the maintained shell instead of claiming full accessibility. Qt fallback is not justified by current evidence.

## Maintained layout and scope

Create a Cargo workspace with `crates/lightwell-core` for read-only JPEG decode/profile/resource rules and bounded open requests, `crates/lightwell-app` for window/native adapters and evidence, and `xtask` for actual build/check/package orchestration. Avoid empty catalog/editor/IPC/plugin crates. Keep the isolated comparison probes for reproducibility, excluded from the root workspace.

The first maintained slice must compile a window using the common core, preserve source bytes, open on a worker, reject stale generations, and display accepted sRGB/greyscale JPEG at Fit. Transfer existing tested logic with equivalent tests before extending it. No editor features. Follow the [platform matrix](../engineering/platforms.md), accepted [bootstrap spec](../specs/bootstrap.md), and [tooling contract](../engineering/development.md).

Next tooling slices add typed errors, bounded logs, run/state identity, explicit requested-versus-presented generations, real renderer captures and process-level smoke checks. Smoke output belongs in a fresh ignored directory; captures must be independently checked. Package only native host artifacts; CI definitions and missing Windows/Linux sessions are not equivalent to executed native checks.

## Profile and resource contract

Use the bounded numeric matrix/TRC recognition proven in the follow-up probe, not fixture-byte matching or a profile-name test. Preserve the documented 128 MiB file, 64 MP, 16384-pixel dimension bounds and 512 MiB decoder-allocation bound. Separate decoded image size from upload preview size; bound preview textures to 4096 pixels per side and resize on the worker before upload. This fits within wgpu's conservative texture limits and avoids full 60 MP UI-thread uploads. S0 is Fit-only, so it needs no source-resolution zoom texture.

One active and one latest pending decode; discard stale completion. No thread join on the GUI thread. Error replacement leaves the last image visible. New decode requests, source checks and native path handling belong in the core/service boundary; framework widgets hold display handles only.

## Acceptance and remaining risks

TASK-006 completes with locked workspace build and testable core boundary, not all S0 tasks. Later tasks require actual command implementations, dependency notices, fixture/evidence checks, native packages, repeatable M4 measurement and M4 desktop evidence plus automated portable checks. macOS floor, Windows/Linux runtime details, complete accessibility and broad ICC/display calibration remain unverified. Manual Windows/Linux and license reviews are now deferred by owner instruction; other limitations must remain explicit.

## Maintained evidence slice

Implement opt-in `--open PATH` (repeatable for a deterministic development sequence), `--evidence-dir DIR`, `--window-size WIDTH HEIGHT` and a bounded deadline. Each sequential request uses the same loader as native Open; capture after the resulting UI frame, including failure states retaining the previous texture. Record requested generation separately from displayed generation, source dimensions separately from bounded preview dimensions, renderer backend and physical scale. Save per-step state/frame records and final events/result/reproduction files in the explicit fresh directory, on the background task executor. Native ordinary mode creates no catalog/config and no source-adjacent files. UI polls only during load/capture/deadline monitoring in explicit evidence mode.

Developer runner: cross-platform Rust xtask, with non-mutating Doctor, formatting/lint/test/check/build/develop, fixture-driven smoke and host package assembly. Fail every subprocess error and classify missing desktop evidence honestly. Packaging is a local unsigned development artifact, not a passing license audit or S0 closure. CI uses the same commands and retains only synthetic fixture artifacts; its configuration is not executed CI evidence.
