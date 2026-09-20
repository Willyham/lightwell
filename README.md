# Lightwell

A developing, open-source, non-destructive desktop photo editor for macOS, Windows, and Linux, targeting an M4 MacBook Pro first. Built around a small, fast core that people, programs, and agents can operate equally.

**Current state: S0 is owner-accepted; M1 history and M2 exact transforms are implemented and locally verified on the M4 Mac.** The Rust/Iced desktop now references JPEGs in a SQLite catalog, preserves ordered edit layers and immutable history, supports pixel proof edits, undo/redo/restore and historical previews, and exposes the same operations through JSONL and authenticated live loopback sessions. Rotate left/right, Mirror horizontal and Flip vertical are exact saved operations. Fresh hosted closure-snapshot verification, manual Windows/Linux desktop checks, native screen-reader verification and manual license reviews remain unfinished follow-ups. All milestones belong to v0.

Run `cargo xtask develop`, or `cargo xtask check` for repository, formatting, lint and unit checks. See [working scaffold commands](docs/engineering/scaffold-commands.md) for setup, smoke evidence and packaging.

- [Project plan](docs/plan.md): recommendation, scope, milestones, and completion gates.
- [Technical research](docs/research/technical-options.md): primary sources, alternatives, and unresolved risks.
- [Lightroom Classic knowledge base](docs/research/lightroom/README.md): sourced technical research on recipes, rendering, performance and editing tools.
- [darktable knowledge base](docs/research/darktable/README.md): source-level explanations of history, pixelpipe, tool algorithms, GPU/cache behavior and automation.
- [Architecture](docs/design/architecture.md): catalog, image engine, commands, and extension boundaries.
- [First build: image-loading skeleton](docs/specs/bootstrap.md): cross-platform launch, Open image and Fit display, with no editing tools.
- [Development and agent verification](docs/engineering/development.md): setup, linting, builds, logs, screenshots, smoke checks and packaging.
- [History-first roadmap](docs/design/history-first-roadmap.md): implemented M1 history/pixel editing and M2 transforms, followed by planned M3 tool modules and M4 crop.
- [Editor specification](docs/specs/single-image.md): staged editing and retained export/recovery/agent follow-ups.
- [Source recovery](docs/specs/source-recovery.md): referenced originals, stable identity, and verified recovery after files move.
- [Performance plan](docs/specs/performance.md): workloads, provisional budgets, and measurement rules.
- [Feature status](docs/features.md): planned now, planned later, and excluded.
- [Product decisions](docs/decisions.md): confirmed requirements and questions for the project owner.
- [User guide](docs/user-guide.md): intended workflow, explicitly distinguished from available behavior.
- [Task sequencing](docs/task-planning.md): active plans, local IDs, dependencies and verification rules.
- [Active task plans](tasks/README.md): independently numbered S0, milestone, follow-up, product-decision and research DAGs.

Read [AGENTS.md](AGENTS.md) before working in this repository. Substantial new features, milestones, migrations and other coordinated multi-step work should start with a Markdown plan/specification and a validated JSON task plan created with the Create Tasks skill. Standalone research, reviews, diagnostics, documentation maintenance and small contained changes do not require task-plan changes unless explicitly requested or already assigned from a tracked task.

The owner prefers an open-source-only project and extension ecosystem. The owner has selected GPL-3.0-or-later. The repository license is applied; manual license/native/asset review remains deferred and incomplete. Initial RAW targets are Nikon Z6 and Fujifilm X100VI.

Start with [contributor instructions](CONTRIBUTING.md), the [native S0 verification](docs/engineering/s0-hardening-results.md) and the [M1/M2 evidence](docs/engineering/m1-m2-results.md). `cargo xtask check-repository` is available now.

Project code is licensed under GNU GPL version 3 or (at your option) any later version: **GPL-3.0-or-later**. See [LICENSE](LICENSE). Third-party components retain their own licenses; the configured dependency review remains in progress.
