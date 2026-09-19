# Contributing to Lightwell

Read [AGENTS.md](AGENTS.md), the [project plan](docs/plan.md), relevant specification and both [active task plans](docs/task-planning.md) before starting. Update the Markdown design before implementation; preserve task IDs/history, and update features and user documentation alongside behavior. Do not execute archived plans or count a prototype as an implemented application feature.

## Repository layout

- `docs/`: specifications, decisions, engineering instructions and measured reports.
- `tasks/`: separate product and implementation plans; `archive/` is immutable history.
- `tools/`: portable repository checks and fixture preparation.
- `fixtures/`: small synthetic inputs with provenance; generated large workloads stay in ignored `fixtures/generated/`.
- `probes/`: bounded experiments, when introduced, separate from the maintained application.
- `artifacts/`: ignored per-run diagnostics, screenshots and measurements.
- `dist/`: ignored development packages.
- `private/`: ignored local originals; never add private files with force.

The maintained source layout is recorded in `docs/design/s0-stack.md`. Do not create empty future subsystems. Platform adapters own native operations; business rules belong in the UI-independent service.

## Current checks

Run from the repository root with the pinned Rust toolchain:

```sh
cargo xtask check-repository
```

This checks local Markdown/file links, both task schemas and DAGs, task status prerequisites, and external product gates. The pinned Rust toolchain runs these checks. The checked-in schema defines the task format; keep it synchronized if the format is deliberately changed. Application formatting/build/test commands are documented below.

Fixture generation has separate pinned tooling; see the [fixture manifest](fixtures/README.md). Routine checks must not rewrite tracked files. Put all run evidence in `artifacts/<run-id>/`; document exact commands, build, fixture hash, backend and capture provenance. A skipped desktop check is not a pass. Do not commit private paths, machine identifiers, screenshots, logs, build outputs or camera originals. Only synthetic/licensed inputs may enter CI.

The owner has selected GPL-3.0-or-later. The repository license text and manifests apply this selection; do not imply the dependency audit has passed. Record actual dependency features, licenses, native runtime requirements and bundled assets before maintained builds/packages are declared complete.

## Maintained scaffold

The root Cargo workspace contains `crates/lightwell-core`, `crates/lightwell-app` and `xtask`. Run `cargo xtask check` before handing off changes. See [actual commands](docs/engineering/scaffold-commands.md) for local execution, evidence and packaging. The isolated `probes/s0` workspace remains experimental.
