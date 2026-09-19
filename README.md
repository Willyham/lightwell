# Lightwell

A planned, open-source, non-destructive desktop photo editor for macOS, Windows, and Linux, targeting an M4 MacBook Pro first. Built around a small, fast core that people, programs, and agents can operate equally.

**Current state: the maintained Rust/Iced scaffold builds and displays JPEGs at Fit on the M4 Mac.** Native Metal smoke checks cover empty, load and failed replacement states. An unsigned local macOS development package is available. Local hardening and an initial M4 baseline are verified. The owner confirmed manual native JPEG opening; S0 remains open for fresh hosted verification. Manual Windows/Linux desktop checks and manual license reviews are deferred. Editing features are not implemented. All milestones belong to v0.

Run `cargo xtask develop`, or `cargo xtask check` for repository, formatting, lint and unit checks. See [working scaffold commands](docs/engineering/scaffold-commands.md) for setup, smoke evidence and packaging.

- [Project plan](docs/plan.md): recommendation, scope, milestones, and completion gates.
- [Technical research](docs/research/technical-options.md): primary sources, alternatives, and unresolved risks.
- [Architecture](docs/design/architecture.md): catalog, image engine, commands, and extension boundaries.
- [First build: image-loading skeleton](docs/specs/bootstrap.md): cross-platform launch, Open image and Fit display, with no editing tools.
- [Development and agent verification](docs/engineering/development.md): setup, linting, builds, logs, screenshots, smoke checks and packaging.
- [Subsequent editor specification](docs/specs/single-image.md): the M1 JPEG import/edit/export experience.
- [Source recovery](docs/specs/source-recovery.md): referenced originals, stable identity, and verified recovery after files move.
- [Performance plan](docs/specs/performance.md): workloads, provisional budgets, and measurement rules.
- [Feature status](docs/features.md): planned now, planned later, and excluded.
- [Decision interview](docs/decisions.md): confirmed requirements and questions for the project owner.
- [User guide](docs/user-guide.md): intended workflow, explicitly distinguished from available behavior.
- [Task sequencing](docs/task-planning.md): active plans, dependencies, decision gates and preserved history.
- [Product decision tasks](tasks/product-decisions.json): unresolved choices and interview work.
- [Implementation tasks](tasks/implementation.json): detailed bootstrap/skeleton work, then the retained editor tasks.

Read [AGENTS.md](AGENTS.md) before working in this repository. Start each change with a Markdown plan/specification, then create or update its validated JSON task plan using the Create Tasks skill.

The owner prefers an open-source-only project and extension ecosystem. The owner has selected GPL-3.0-or-later. The repository license is applied; the configured dependency audit remains TASK-041. Initial RAW targets are Nikon Z6 and Fujifilm X100VI.

Start with [contributor instructions](CONTRIBUTING.md) and the [measured S0 probe results](docs/engineering/s0-probe-results.md). `python3 tools/check_repository.py` is available now.

Project code is licensed under GNU GPL version 3 or (at your option) any later version: **GPL-3.0-or-later**. See [LICENSE](LICENSE). Third-party components retain their own licenses; the configured dependency review remains in progress.
