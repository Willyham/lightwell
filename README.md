# Lightwell

A planned, open-source, non-destructive desktop photo editor for macOS, Windows, and Linux, targeting an M4 MacBook Pro first. Built around a small, fast core that people, programs, and agents can operate equally.

**Current state: S0 is owner-accepted, and the maintained Rust/Iced scaffold builds and displays JPEGs at Fit on the M4 Mac.** Native Metal smoke checks cover empty, load and failed replacement states. An unsigned local macOS development package is available. Local hardening and an initial M4 baseline are verified. Fresh hosted closure-snapshot verification, manual Windows/Linux desktop checks and manual license reviews remain unfinished follow-ups. Editing features are not implemented. All milestones belong to v0.

Run `cargo xtask develop`, or `cargo xtask check` for repository, formatting, lint and unit checks. See [working scaffold commands](docs/engineering/scaffold-commands.md) for setup, smoke evidence and packaging.

- [Project plan](docs/plan.md): recommendation, scope, milestones, and completion gates.
- [Technical research](docs/research/technical-options.md): primary sources, alternatives, and unresolved risks.
- [Lightroom Classic knowledge base](docs/research/lightroom/README.md): sourced technical research on recipes, rendering, performance and editing tools.
- [darktable knowledge base](docs/research/darktable/README.md): source-level explanations of history, pixelpipe, tool algorithms, GPU/cache behavior and automation.
- [Architecture](docs/design/architecture.md): catalog, image engine, commands, and extension boundaries.
- [First build: image-loading skeleton](docs/specs/bootstrap.md): cross-platform launch, Open image and Fit display, with no editing tools.
- [Development and agent verification](docs/engineering/development.md): setup, linting, builds, logs, screenshots, smoke checks and packaging.
- [History-first roadmap](docs/design/history-first-roadmap.md): M1 history/pixel editing, M2 transforms, M3 tool modules and M4 crop; implementation remains on hold.
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

Start with [contributor instructions](CONTRIBUTING.md) and the [native S0 verification](docs/engineering/s0-hardening-results.md). `cargo xtask check-repository` is available now.

Project code is licensed under GNU GPL version 3 or (at your option) any later version: **GPL-3.0-or-later**. See [LICENSE](LICENSE). Third-party components retain their own licenses; the configured dependency review remains in progress.
