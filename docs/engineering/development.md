# Development, builds and agent verification

Status: **maintained tooling implemented; S0 accepted and M1/M2 locally verified with outstanding hosted/platform follow-ups**. See [contributing](../../CONTRIBUTING.md), [scaffold commands](scaffold-commands.md) and [M1/M2 results](m1-m2-results.md). The operation table below defines the maintained-app contract.

## Repository and toolchain

The selected Rust workspace has an application crate, a UI-independent core and xtask orchestration. M1 persistence uses bundled SQLite through pinned `rusqlite`. The application lockfile and Rust toolchain remain pinned; native SDK/runtime prerequisites are recorded per OS. [Cargo workspace reference](https://doc.rust-lang.org/cargo/reference/workspaces.html).

Repository setup should cover `.gitignore`, line endings, editor defaults, generated-file locations, fixture provenance, contribution steps and documentation links. Do not check in private photos, build products, logs or screenshots. Keep checked-in synthetic fixtures small and reproducible. Retain only the scaffolding needed by the skeleton.

Provide platform-specific setup instructions and a non-mutating environment check that reports missing prerequisites and useful installation guidance. Do not silently install SDKs, modify shell profiles, enable global hooks or require a container to run the desktop app. Optional editor settings and local watch workflows must not be required by CI.

## One documented command surface

The following list defines the required tooling outcomes. The maintained Rust `xtask` now implements the commands listed in [scaffold commands](scaffold-commands.md); broader acceptance requirements below remain in force:

| Operation | Required behavior |
| --- | --- |
| Doctor | Report toolchain, native prerequisites, graphics/display environment and actionable gaps |
| Develop | Launch a development build, optionally with a fixture and isolated state |
| Format / lint | Check formatting, compiler/linter diagnostics and documentation/task-plan consistency |
| Test | Run UI-independent correctness tests without a display |
| Build | Produce debug/release builds with the locked dependency graph |
| Smoke | Run one scenario with a deadline, structured status, logs and capture artifacts |
| Package | Assemble the host's native development artifact and notices |
| Check | Run the normal local/CI checks and distinguish unavailable GUI checks from passes |

Handle spaces, Unicode and platform path conventions correctly. Share orchestration logic between local development and CI; avoid separate undocumented shell pipelines. Keep exit codes meaningful, support per-run temporary directories, and document which operations require network dependency downloads or a graphical session. Reproducible setup means pinned inputs and repeatable steps; do not claim byte-for-byte reproducible native binaries without measuring them.

## Formatting, linting and dependency checks

Use rustfmt and Clippy from the pinned compiler toolchain if Rust is selected; fail on relevant warnings in CI with narrowly justified local exceptions. The Clippy project recommends CI enforcement with `-Dwarnings` and matching the compiler toolchain. Do not adopt blanket pedantic lints that obscure useful diagnostics. [Clippy CI guidance](https://doc.rust-lang.org/clippy/continuous_integration/index.html).

Check dependency licenses against the selected project policy, inspect native libraries/assets and optional features, and report known advisories with narrowly scoped, documented exceptions. Record the exact selected check tool and version; do not depend on a developer's globally installed binary. Check Markdown local links and every active task plan's schema, local numbering and DAG. Optional pre-commit hooks should reuse these commands.

## Logs and state

Use structured events from the service/decoder/renderer, with a human-readable developer view and JSONL output for agents. Rust's `tracing`/`tracing-subscriber` is a candidate; it supports events/spans and newline-delimited JSON formatting. Configure output deliberately: diagnostics go to stderr/files so future JSON/MCP stdout stays clean. [Structured logging reference](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/fmt/index.html).

Include run ID, event name, severity, monotonic elapsed time, request/generation ID, build identity, stage timings and error codes. Emit startup/backend information once rather than on every frame. Capture controlled panic/exit diagnostics without claiming recovery from an unrecoverable process crash. Log I/O failure must not crash ordinary image viewing; explain degraded evidence collection. Bound file size/retention and disable noisy per-frame logging by default.

Routine runs use fixture/asset identifiers instead of private absolute paths or embedded EXIF/GPS values. Diagnostic verbosity and local path inclusion are explicit. No network telemetry is required.

## Screenshot and result contract

Each smoke run uses its own ignored artifact directory, proposed as `artifacts/<run-id>/`, containing `result.json`, `events.jsonl`, `state.json`, capture PNGs when available, subprocess output and a Markdown reproduction note. The future result schema records scenario, build, OS/architecture, backend/adapter, display scale, fixture hash, request generation, readiness stage, timing, exit status and capture provenance. Stable keys help agents inspect results; this remains an internal v0 format.

Capture after the intended frame is ready, not merely after the decoder succeeds. Capture the app's actual render target/content including its UI where supported. Store physical dimensions, scale, color interpretation and whether the capture is offscreen, renderer readback or an OS window screenshot. Confirm a blank, stale or failed render fails the smoke check; existence of a PNG is insufficient.

Separate assertions:

- State checks: requested fixture, dimensions, orientation, status and generation match.
- Pixel checks: expected image region, aspect/orientation and representative colors/detail match declared tolerances. JPEG and platform text rendering do not require universal byte equality.
- UI review: inspect screenshot layout, clipping, empty/loading/error states and visual hierarchy.
- Native checks: exercise dialogs, keyboard focus, resizing, OS presentation and shutdown in a real desktop session.

Compare deterministic fixtures using geometry/region assertions and documented tolerances. Store per-platform baselines only when necessary; a reviewed baseline change must not conceal a functional regression. Extend this harness through the history, transform, module and crop milestones, then the retained export/MCP follow-ups. Each stage checks UI/API parity.

## CI and packaging

Run dependency-locked compile/unit/lint checks on the selected macOS, Windows and Linux targets. Keep caches separate by toolchain/target/profile and verify clean builds periodically. Build artifacts include target architecture, build identity, runtime dependencies, licenses and checksums. Native packages must launch without a developer toolchain; write down external runtime requirements instead of assuming a fully static binary.

Run GUI tests only in environments with the necessary desktop/GPU access. Distinguish hardware, virtual and software adapters in results. A skipped or unsupported smoke run never counts as native platform verification. Keep a recorded manual/native check when hosted runners cannot supply a suitable desktop session.

If GitHub Actions is selected, store build/test evidence as workflow artifacts, including failure output, with explicit retention. Equivalent open-source/self-hosted CI must be able to call the same runner; no proprietary hosted service is required to build or test Lightwell. [Workflow artifact documentation](https://docs.github.com/en/actions/tutorials/store-and-share-data).

Automated uploads must use synthetic fixtures and the designated evidence directory. Package/run with isolated writable data directories to avoid a developer's real library. Public signing, notarization, stores and auto-updates are later distribution work.

## Agent execution loop

1. Read the applicable spec and active JSON task, including external product decision gates.
2. Run Doctor and the smallest checks appropriate to the change.
3. For UI/image changes, run a reproducible smoke scenario and read the result/state/logs; inspect the capture as an image.
4. Fix failures and rerun affected checks. Record native checks separately from headless evidence.
5. For changes under `crates/`, answer the [performance rules](performance-rules.md) checklist and run `editor-performance` on a generated 24 MP input in release before claiming a performance result.
6. Report exact commands, artifact paths, results and remaining unsupported cases; update task/feature state only when acceptance is met.

No undocumented clicking or guessed screen coordinates should be required to determine whether an image loaded correctly. The S0 evidence harness is not the editor command API or MCP server. M1 provides a separate working JSONL owner/API and live loopback transport over the same production service; MCP remains planned.

## Maintained workspace setup

The selected S0 workspace uses Rust **1.94.0**, pinned in `rust-toolchain.toml`, with rustfmt/Clippy and a committed Cargo.lock. Install that toolchain explicitly with `rustup toolchain install 1.94.0 --profile minimal --component rustfmt --component clippy`. Native requirements and outstanding platform checks are in [platforms](platforms.md). macOS builds set a 14.0 deployment target unless deliberately overridden; floor execution remains to verify.

Implemented commands and flags are listed in [scaffold commands](scaffold-commands.md): Doctor, check, fmt, lint, test, build, develop, smoke, inventory, audit and host packaging. Use `cargo xtask develop --open "path to/image.jpg"`; image/capture paths require named flags. Cargo needs network on the first locked fetch; subsequent builds can be offline. The Rust runner works on all three targets.

Crates: `lightwell-core` owns images, recipes, rendering, SQLite history, preview scheduling and the JSON API; `lightwell-app` owns the Iced adapter plus desktop/headless binaries; `xtask` owns developer command orchestration and exact editor acceptance. Probes remain outside this workspace. Error/state/log evidence is recorded in the S0 and M1/M2 engineering reports. Native macOS setup has been exercised; Windows/Linux setup instructions remain unverified and are not represented as passing tests.
