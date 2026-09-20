# Maintained scaffold commands and evidence

S0 is accepted. The Rust/Iced viewer and native M4 Metal checks are implemented; editor work is planned and on hold. Fresh hosted closure-snapshot verification and native Windows/Linux desktop checks remain unfinished.

Use the [bootstrap playbook](bootstrap-playbook.md) for fresh-checkout setup and evidence inspection. See [stack boundaries](../design/s0-stack.md) and [platform prerequisites](platforms.md).

## Commands available now

Run from the repository root with pinned Rust 1.94.0:

```sh
cargo xtask doctor
cargo xtask check
cargo xtask build --release
cargo xtask develop --open fixtures/s0/orientation-6.jpg
# Explicit unoptimized debugging (unsuitable for timing):
cargo xtask develop --debug --open fixtures/s0/orientation-6.jpg
cargo xtask smoke --scenario load --output artifacts/new-load
cargo xtask smoke --scenario replacement --output artifacts/new-replacement
cargo xtask smoke --scenario empty --output artifacts/new-empty
cargo xtask inventory --output artifacts/new-inventory
cargo xtask package --output artifacts/new-package
```

Smoke scenarios are `empty`, `load`, `replacement`, `invalid`, `repeated`, `alternating`, `large24` and `large60`; generate large fixtures with `cargo xtask generate-fixtures` into a new `fixtures/generated` directory before the last two.

Smoke requires a native graphical session; image checks are built into Rust xtask. Use a fresh output directory each time; refusing existing directories prevents stale evidence. `fmt`, `lint`, `test` and `build` are individually callable through xtask. `check` runs plan/link checks, formatting, Clippy and tests; it explicitly does not imply graphical or dependency-audit acceptance. Doctor reports missing tools without installing them. Setup installation commands remain explicit in [development](development.md).

The application accepts `--data-root DIRECTORY` for isolated diagnostic/config/cache paths, `--open PATH`, `--window-size WIDTH HEIGHT` (logical dimensions 320..4096), and `--evidence-dir NEW_DIRECTORY`. Repeat `--open` only in evidence mode to run a bounded development sequence. Normal mode shows native Open with Cmd+O on macOS and Ctrl+O on Windows/Linux; no data/catalog writes are needed. Source paths stay native OS paths. No working public editing API or MCP is implied.

Evidence mode disables manual opening to keep the sequence deterministic. It uses the same image loader, writes state and real window-renderer PNGs per request, then final `events.jsonl`, `state.json`, and `result.json`. PNG encoding and evidence finalization run on the task executor, away from the UI thread. Application status `captured` is not a pixel-test pass: the outer smoke runner verifies fixture colors, Fit, generation/state, backend, exit status and source SHA-256 before writing its own `passed` result. Each bundle also includes subprocess output and reproduction arguments. Errors retain the previous displayed generation and photo. A 25-second app deadline and 35-second process deadline bound hangs.

## Implemented checks and evidence

Core tests cover EXIF orientations, supported/unsupported profiles, malformed/oversized/missing/read-only sources, unchanged input bytes, latest-request scheduling and the 4096-pixel preview bound. App tests cover failed replacement/cancel retention, isolated paths, diagnostic failures and renderer readiness.

Incremental background logs carry run/request identities. Explicit GPU allocation readiness precedes capture. The process runner rejects blank/stale/missing evidence, times out and reaps hung children, and records binary/lockfile hashes and reproduction arguments. Eight native M4 Metal scenarios and adversarial failure checks pass; [hardening results](s0-hardening-results.md) contain exact tested identities, measurements and native picker limitations.

The owner confirmed manual JPEG opening. Automated picker selection is not claimed to pass. Custom-control accessibility, calibrated display-color management, minimum-OS execution and native Windows/Linux behavior remain unverified.

Ordinary viewing writes no catalog/config/cache or source-adjacent files. It logs to stderr unless an explicit data root requests isolated diagnostics; existing logs are not overwritten.

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
