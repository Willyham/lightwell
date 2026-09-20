# Maintained scaffold commands and evidence

S0 is accepted. The Rust/Iced viewer plus M1 history and M2 exact transforms are implemented and locally verified on native M4. Fresh hosted closure-snapshot verification and native Windows/Linux desktop checks remain unfinished.

Use the [bootstrap playbook](bootstrap-playbook.md) for fresh-checkout setup and evidence inspection. See [stack boundaries](../design/s0-stack.md) and [platform prerequisites](platforms.md).

## Commands available now

Run from the repository root with pinned Rust 1.94.0:

```sh
cargo xtask doctor
cargo xtask check
cargo xtask build --release
cargo xtask develop --catalog /tmp/lightwell-catalog.sqlite --open fixtures/s0/orientation-6.jpg
# Explicit unoptimized debugging (unsuitable for timing):
cargo xtask develop --debug --open fixtures/s0/orientation-6.jpg
cargo run --release --locked --package xtask -- editor-acceptance --output artifacts/new-editor-acceptance
cargo run --release --locked --package xtask -- editor-performance --source fixtures/generated/24mp.jpg --output artifacts/new-editor-performance --samples 10
cargo xtask smoke --scenario load --output artifacts/new-load
cargo xtask smoke --scenario replacement --output artifacts/new-replacement
cargo xtask smoke --scenario empty --output artifacts/new-empty
cargo xtask inventory --output artifacts/new-inventory
cargo xtask package --output artifacts/new-package
```

Smoke scenarios are `empty`, `load`, `replacement`, `invalid`, `repeated`, `alternating`, `large24` and `large60`; generate large fixtures with `cargo xtask generate-fixtures` into a new `fixtures/generated` directory before the last two.

Smoke requires a native graphical session; image checks are built into Rust xtask. Use a fresh output directory each time; refusing existing directories prevents stale evidence. `editor-acceptance` is display-independent and exercises the complete persistent M1/M2 journey with exact buffers, 208 paged entries and catalog reopen. `editor-performance` records core source-cache and one/200-transform timings for an explicit JPEG; it excludes desktop scheduling, GPU upload and presentation. Run timing commands in release mode. `fmt`, `lint`, `test` and `build` are individually callable through xtask. `check` runs plan/link checks, formatting, Clippy and tests; it explicitly does not imply graphical or dependency-audit acceptance. Doctor reports missing tools without installing them. Setup installation commands remain explicit in [development](development.md).

The application accepts `--catalog FILE`, `--data-root DIRECTORY`, `--open PATH`, `--window-size WIDTH HEIGHT` (logical dimensions 320..4096), and `--evidence-dir NEW_DIRECTORY`. Repeat `--open` only in S0 evidence mode to run a bounded development sequence. Normal mode is the M1/M2 editor: it owns the catalog, shows native Open with Cmd+O on macOS and Ctrl+O on Windows/Linux, and starts an authenticated loopback JSON service. Source paths stay native OS paths. The working JSON API is internal v0 and is not MCP.

Build the headless JSONL owner with `cargo xtask build --release`, then use:

```sh
target/release/lightwell-json --catalog /path/to/catalog.sqlite < requests.jsonl
```

Start with `schema.list`; request/response examples and live-session behavior are in the [user guide](../user-guide.md). Only one process owns a catalog at a time.

Evidence mode disables manual opening to keep the sequence deterministic. It uses the same image loader, writes state and real window-renderer PNGs per request, then final `events.jsonl`, `state.json`, and `result.json`. PNG encoding and evidence finalization run on the task executor, away from the UI thread. Application status `captured` is not a pixel-test pass: the outer smoke runner verifies fixture colors, Fit, generation/state, backend, exit status and source SHA-256 before writing its own `passed` result. Each bundle also includes subprocess output and reproduction arguments. Errors retain the previous displayed generation and photo. A 25-second app deadline and 35-second process deadline bound hangs.

## Implemented checks and evidence

Core tests cover EXIF orientations, supported/unsupported profiles, malformed/oversized/missing/read-only sources, unchanged input bytes, exact pixel/transform recipes, atomic SQLite recovery, persistent navigation, API sessions and bounded render scheduling. App tests cover editor keyboard mapping plus the S0 failed-replacement/cancel, isolated-path, diagnostic and renderer-readiness cases. A subprocess test proves the headless JSON owner handles multiple requests and clean EOF.

Incremental background logs carry run/request identities. Explicit GPU allocation readiness precedes capture. The process runner rejects blank/stale/missing evidence, times out and reaps hung children, and records binary/lockfile hashes and reproduction arguments. Eight native M4 Metal scenarios and adversarial failure checks pass; [hardening results](s0-hardening-results.md) contain exact tested identities, measurements and native picker limitations.

The owner confirmed manual JPEG opening. Automated picker selection is not claimed to pass. M1/M2's packaged native journey, live-client edit, restart, keyboard undo/redo and resource observations are recorded in [M1/M2 results](m1-m2-results.md). Native screen-reader exposure for custom controls, calibrated display-color management, minimum-OS execution and native Windows/Linux behavior remain unverified.

Editor mode writes the selected/default catalog and a temporary adjacent live-session descriptor, but never writes the source JPEG. S0 evidence mode retains its isolated artifact behavior. Diagnostics use stderr unless an explicit data root requests isolated logs; existing logs are not overwritten.

## Packaging and CI

`cargo xtask package` creates unsigned host development artifacts with package identities, dependency inventory and copied notices. Inventory is not a completed license audit. No public signing/notarization, deployment or store submission is provided.

The CI workflow runs shared locked checks/build/package jobs on macOS, Windows and Ubuntu, with seven-day artifacts, fixture and blocking audit jobs. Linux renderer checks use Xvfb/software Vulkan. The verified baseline, latest local package and outstanding hosted execution are identified in [CI results](ci-results.md). Hosted/headless results do not establish native desktop or GPU performance.

## Dependency policy

Install the pinned audit tool locally once (requires network):

```sh
cargo install --locked --version 0.20.2 --root .tools/cargo-deny cargo-deny
cargo xtask audit
```

Audit validates [exact-version, expiring advisory exceptions](advisory-policy.md) and fails on expired/changed exceptions or unapproved findings. The base `deny.toml` is strict. Current review results and incomplete manual asset/native notices are in [dependency review](dependency-review.md). Audit is separate from headless `check`.

## Performance and tooling

Default `develop` uses release optimization. Explicit `develop --debug` and plain `cargo run` are unoptimized and unsuitable for performance measurements. Stage diagnostics identify read, validation, decode, orientation, resize, RGBA and upload costs; see [JPEG measurements](jpeg-performance.md).

All maintained commands run in Rust. [Rust tooling](rust-tooling.md) includes fixture generation, capture inspection, process failure checks and macOS measurement. Measurements stay tied to the tested binary, input and capture provenance; rerun affected checks after code changes.
