# Maintained scaffold commands and evidence

Status: runnable macOS scaffold with initial developer tooling; **S0 acceptance is not complete**. Stack rationale is in [S0 stack](../design/s0-stack.md), native requirements in [platforms](platforms.md), and earlier experiments in [probe results](s0-probe-results.md).

## Commands available now

Run from the repository root with pinned Rust 1.94.0 and Python 3.10+:

```sh
cargo xtask doctor
cargo xtask check
cargo xtask build --release
cargo xtask develop --open fixtures/s0/orientation-6.jpg
cargo xtask smoke --scenario load --output artifacts/new-load
cargo xtask smoke --scenario replacement --output artifacts/new-replacement
cargo xtask smoke --scenario empty --output artifacts/new-empty
cargo xtask inventory --output artifacts/new-inventory
cargo xtask package --output artifacts/new-package
```

Smoke additionally requires Pillow 12.2.0 from [fixture requirements](../../tools/fixture-requirements.txt), and a native graphical session. Use a fresh output directory each time; refusing existing directories prevents stale evidence. `fmt`, `lint`, `test` and `build` are individually callable through xtask. `check` runs plan/link checks, formatting, Clippy and tests; it explicitly does not imply graphical or dependency-audit acceptance. Doctor reports missing tools without installing them. Setup installation commands remain explicit in [development](development.md).

The application accepts `--open PATH`, `--window-size WIDTH HEIGHT` (logical dimensions 320..4096), and `--evidence-dir NEW_DIRECTORY`. Repeat `--open` only in evidence mode to run a bounded development sequence. Normal mode shows native Open with Cmd+O on macOS and Ctrl+O on Windows/Linux; no data/catalog writes are needed. Source paths stay native OS paths. No working public editing API or MCP is implied.

Evidence mode disables manual opening to keep the sequence deterministic. It uses the same image loader, writes state and real window-renderer PNGs per request, then final `events.jsonl`, `state.json`, and `result.json`. PNG encoding and evidence finalization run on the task executor, away from the UI thread. Application status `captured` is not a pixel-test pass: the outer smoke runner verifies fixture colors, Fit, generation/state, backend, exit status and source SHA-256 before writing its own `passed` result. Each bundle also includes subprocess output and reproduction arguments. Errors retain the previous displayed generation and photo. A 25-second app deadline and 35-second process deadline bound hangs.

## Current verification

The root workspace compiled and passed Clippy/format/core tests on the M4. Native Metal load, empty and invalid-replacement smoke scenarios passed in `artifacts/maintained-load-1`, `artifacts/maintained-empty-1`, and `artifacts/maintained-replacement-1`; 1920×1280 renderer captures at 2× were independently pixel-checked and visually reviewed. Source hashes were unchanged. Subsequent user-facing error wording was made clearer; repeat affected smoke checks after code changes before treating those prior frames as current evidence.

Core tests cover all EXIF orientations, accepted and unsupported profiles, malformed/oversized/missing files, preservation and newest-request filtering. Preview tests enforce the 4096-pixel upload bound independently of original dimensions. Original decoder allocation limits are not whole-process memory budgets; CPU/GPU copies and capture buffers are additional bounded allocations.

Native Iced probe interaction verified Cmd+O, picker load, cancellation and resize/Fit. Native automated invalid-file selection became unreliable; the maintained process smoke test now verifies the shared failed-replacement path and its actual rendered result. It does not claim another successful manual picker check. AX exposure of custom controls remains limited; full accessibility is not established.

## Pending work and limitations

Structured evidence is initial developer tooling, not completion of TASK-044/048/051: crash-time incremental logs, run/build identity, controlled diagnostic-write failure, broader malformed/read-only cases and deliberately hung/blank-process smoke tests need further work. Error categories are still strings; TASK-042 remains open for typed errors and the broader data-path contract. Ordinary mode currently writes no persistent configuration/cache/logs. No hidden catalog or compatibility layer has been added.

Packaging assembles unsigned host development artifacts plus license inventory and copied notices. Inventory lists resolved host dependency licenses but is not a license/advisory audit; bundled assets/native runtime review remains required. No public signing/notarization, deployment, upload or store submission has occurred.

The checked-in CI workflow runs locked checks/build/package on macOS, Windows and Ubuntu and retains artifacts for seven days. It has **not been executed** in this session. Hosted compile runners are not the required native desktop launch/load checks. Windows 2022 CI build is not Windows 11 user-session evidence. S0 remains open until the accepted platform matrix and remaining local hardening/performance tasks pass.

## Dependency policy and latest evidence

Install the pinned audit tool locally once (requires network):

```sh
cargo install --locked --version 0.20.2 --root .tools/cargo-deny cargo-deny
cargo xtask audit
```

Audit returns failure for unresolved findings; see [dependency review](dependency-review.md). License/source checks pass, but two maintenance advisories and the full embedded asset/native notice review remain open. This command is intentionally separate from headless `check` and is not represented as passing CI.

The host package was built at `artifacts/maintained-package-2/lightwell-development.zip`, with SHA-256 in `checksums.txt`. Its actual app binary passed native Metal replacement smoke using an output directory containing spaces (`artifacts/package smoke 2`). The failed-input capture was visually reviewed: the oriented image remains visible and a readable error replaces the loading status. Earlier empty/load/replacement runs live in `artifacts/maintained-{empty,load,replacement}-1`. These paths are ignored local evidence, not checked-in portable reports. A subsequent bounded CLI change limits evidence sequences to 16 requests; rerun package/smoke after changes rather than treating an old archive as current.

The runner also verifies a missing executable produces exit 1 and `status: failed`, without a fabricated screenshot. New smoke results record binary and lockfile hashes. Renderer readback proves content; native picker/focus/desktop observations remain separate. Ordinary viewing creates no catalog, config or source-adjacent files. Diagnostics hardening, typed errors, native accessibility, full lifecycle/resource measurements and Windows/Linux sessions remain required before completing S0.

Latest package: `artifacts/maintained-package-3/lightwell-development.zip`. Its replacement smoke passed in `artifacts/package smoke 3` with binary/lockfile hashes recorded. This includes the 16-request limit and friendly error messages. No Windows machine is currently available, as confirmed by the owner.
