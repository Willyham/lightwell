# Development, verification and packaging

All tooling is Rust: `cargo xtask <command>`. Commands reject unknown arguments and pass paths to child processes without shell interpolation. `cargo xtask help` lists everything.

## Setup

Install Git and Rust through rustup plus the platform prerequisites in [platforms](platforms.md). Nothing here installs system tools silently.

```sh
rustup toolchain install 1.94.0 --profile minimal --component rustfmt --component clippy
cargo xtask doctor
cargo xtask check
cargo xtask build --release
```

Doctor reports missing tools and the graphics environment without installing anything; it does not prove a desktop or GPU is available. The first build fetches pinned crates and needs network.

## Commands

| Purpose | Command |
| --- | --- |
| Environment report | `cargo xtask doctor` |
| Full local and CI checks: repository links and task plans, formatting, Clippy, tests | `cargo xtask check` |
| Individual steps | `cargo xtask check-repository`, `fmt`, `lint`, `test`, `build [--release]` |
| Run the editor, release build | `cargo xtask develop [--catalog FILE] [--open PATH] [--data-root DIR]` |
| Run an unoptimized build, debugging only | `cargo xtask develop --debug ...` |
| Exact M1/M2 journey, display-independent | `cargo run --release --locked --package xtask -- editor-acceptance --output NEW_DIR` |
| Core timing on a real-sized JPEG | `cargo run --release --locked --package xtask -- editor-performance --source JPEG --output NEW_DIR [--samples N]` |
| Verify golden fixtures; generate 24 and 60 MP workloads | `cargo xtask fixtures`, `cargo xtask generate-fixtures [--output NEW_DIR]` |
| Rendered smoke scenario, needs a native graphical session | `cargo xtask smoke --scenario NAME --output NEW_DIR [--binary PATH]` |
| Inspect a capture | `cargo xtask check-capture --image PNG [--orientation N]` |
| Process failure checks; macOS measurement | `cargo xtask hardening --binary PATH --output NEW_DIR`, `cargo xtask measure --binary PATH --output NEW_DIR [--samples N]` |
| Package; dependency inventory | `cargo xtask package --output NEW_DIR`, `cargo xtask inventory --output NEW_DIR` |
| License, source and advisory policy | `cargo xtask audit`, see [dependencies](dependencies.md) |
| Isolated UI probes | `cargo xtask probe --candidate iced|egui --output NEW_DIR` |

Every evidence command refuses an existing output directory: use a fresh `artifacts/<run-id>/`. Timing commands must use release builds. A debug build makes image work roughly thirty times slower (a 10 MB JPEG took ten seconds to open), which is why `develop` defaults to release. `check` never implies graphical or dependency-audit acceptance.

## Running the application

`cargo xtask develop` starts the editor. It owns the catalog (`--catalog FILE`, defaulting to the platform configuration directory), offers native Open with Cmd+O or Ctrl+O and starts an authenticated loopback JSON service. `--data-root DIR` isolates config, cache and log paths. The application also accepts `--window-size W H` (320 to 4096 logical) and `--evidence-dir NEW_DIR`. Evidence mode is the same editor driven by the harness: each `--open` goes through the ordinary import call into a catalog created inside the new evidence directory, a window frame is captured after each outcome, and the run exits after writing its results. Manual Open is disabled during collection, and `--open` may repeat only with `--evidence-dir`.

The headless owner reads one JSON request per line:

```sh
target/release/lightwell-json --catalog /path/to/catalog.sqlite < requests.jsonl
```

Start with `schema.list`. Request shapes and live-session behavior are in the [user guide](../user-guide.md). Only one process owns a catalog at a time; a second instance exits with an explanatory error. Diagnostics go to stderr, or to isolated logs under an explicit data root, never to protocol stdout. Editor mode writes only the catalog and a temporary live-session file beside it; the source JPEG is never written.

## Rendered evidence

Smoke runs the built or packaged editor through a deterministic evidence sequence (repeated `--open`, evidence directory, fixed window size, bounded deadlines); there is no separate viewer, so the captured frame is the editor window with its sidebar. Scenarios: `empty`, `load`, `replacement`, `invalid`, `repeated`, `alternating`, `large24`, `large60`; generate the large fixtures first. Each run writes `result.json`, `app/events.jsonl`, `app/state.json`, `app/frame-*.png` (window-renderer readbacks, not OS screenshots), `subprocess.log` and `reproduce.md`. Each frame records `surface_columns`, the physical x range of the photo surface derived from the editor's layout constants, and the runner verifies fixture colors, Fit geometry and centering within that range, generation and state, backend, exit status and unchanged source hashes before writing `passed`; blank, stale or missing frames fail. The `render_ready` event marks the upload of the open request's preview raster, which is when a frame becomes capturable. A 25-second application deadline and a 35-second process deadline bound hangs.

Rules for any UI or image check:

- Capture after the intended generation is rendered, tied to state and logs, with explicit provenance. A PNG's existence is not a pass.
- Keep state checks, pixel checks with declared tolerances, UI review and native checks (dialogs, focus, resize, shutdown in a real desktop session) separate.
- Never report a screenshot as taken when capture is unsupported. A skipped or headless run is not native platform verification.
- Only synthetic fixtures in CI and shared artifacts. Routine logs use fixture identifiers, not private paths or EXIF. Keep personal photos in ignored `fixtures/jpg/` or `private/`.
- Record what was measured: host, build profile, fixture hash, backend, warm or cold cache. Native M4 timings are hardware evidence; VM or Xvfb runs are functional evidence only.

## Agent loop

1. Read the applicable spec and task, including any owner-decision gates.
2. Run `doctor` and the smallest checks appropriate to the change.
3. For UI or image changes, run a smoke scenario or the acceptance journey and inspect the capture as an image.
4. For changes under `crates/`, answer the [performance rules](performance-rules.md) checklist and run `editor-performance` on a generated 24 MP input in release.
5. Report exact commands, artifact paths, results and unsupported cases. Update task and feature status only when acceptance is met.

## Packaging

`cargo xtask package` builds an unsigned host development artifact: a ZIP on macOS and Windows or a `.tar.gz` on Linux containing `Lightwell/` with the executable, notices, `build.json` (source revision, dirty state, target, profile, binary and lockfile hashes) and `checksums.txt`. Run smoke against the packaged executable with `--binary`. Packaging is repeatable, not byte-reproducible, and inventory is not a completed license audit. macOS bundles are unsigned and not notarized. No signing, stores or auto-update exist.

## CI

`.github/workflows/check.yml` runs `cargo xtask check`, an optimized build and packaging on macOS arm64, Windows x64 and Ubuntu x64 with seven-day artifact retention, plus separate fixture and dependency-policy jobs. Linux additionally runs every smoke scenario against the packaged binary under Xvfb with software Vulkan and records runtime imports. Hosted results are compilation and functional evidence, never native desktop or GPU acceptance. Inspect actual run results for the tested commit; a configured step is not a passing result. Fresh hosted verification of the current tree and manual Windows/Linux desktop checks are open items in the [S0 follow-ups](../../tasks/implementation-s0.json).
