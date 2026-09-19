# Working on Lightwell

## Product constraints

- Build a non-destructive photo editor for macOS, Windows, and Linux, serving professional and prosumer collections.
- Target the owner's M4 MacBook Pro first. Keep Windows/Linux portability visible; distinguish VM functional checks from native GPU performance evidence.
- The first end-to-end build is S0: a cross-platform skeleton that opens and displays a JPEG at Fit. No editing tools, catalog, export or production MCP are required in S0. Current owner override: verify native launch/load on M4 plus automated portable checks before marking S0 complete; manual Windows/Linux desktop checks are deferred; M1 adds the editor.
- Prefer open-source-only project code and extensions. The owner has selected GPL-3.0-or-later. Applied in TASK-005; TASK-041 license reviews are deferred by the owner for now, and selection does not imply an audit is complete.
- Initial RAW compatibility targets are Nikon Z6 and Fujifilm X100VI. Benchmark established libraries before proposing a new decoder; custom work requires a documented support or performance gap.
- Preserve original files. Keep edits as data and use a common command layer for UI and programmatic operations.
- Every application operation must be programmable, including every bundled/external edit tool, future masks/clone strokes, settings and module lifecycle. Add discoverable schemas and state access with the feature; GUI gestures cannot be its only interface.
- Keep a small shell/module host: the core owns recipe transactions, history/undo, shared invariants and bounded services; feature modules own their validation and processing through those APIs. Prefer useful optional modules and lazy resource initialization; require measurements before splitting basic core functionality into separately loaded binaries. External module loading remains required later, even if built-ins stay linked. See `docs/design/modules-and-api.md`; do not add a plugin runtime to S0/M1 or silently drop edits when a provider is disabled/missing.
- M1 must support live agent edits while the GUI is open through local IPC/MCP, plus manual Locate for moved originals. Import references existing files; stable asset IDs and edits must survive path changes. Folder relinking and sync workflow research follow later.
- Agreed M1 controls include free crop handles, proportional Option scaling around the fixed center, composition-preserving straightening, and zoom/pan with percentages and Fit. Export strips optional metadata by default and offers a Keep metadata setting; output color and geometry must remain correct in both modes.
- Treat responsiveness, bounded memory, image correctness, and agent access as architectural requirements.
- Prefer a small, beautiful core with sensible defaults and deliberate extension points. Use Lightroom Library/Develop as a familiarity reference, not a feature checklist.
- Consult the owner on consequential product tradeoffs. Record recommendations as proposals until decided; do not turn unanswered interview questions into accepted decisions.

## Documentation-first workflow

1. Read `docs/plan.md`, `docs/features.md`, `docs/decisions.md`, `docs/task-planning.md`, and the relevant specification and task plan.
2. Before implementation, create or update a Markdown design/plan describing behavior, scope, constraints, acceptance criteria, and unresolved decisions.
3. Use the Create Tasks skill to update `tasks/product-decisions.json` and `tasks/implementation.json` separately; preserve task IDs/history and validate both graphs. Do not execute archived plans. Check the documented external product prerequisites before completing an implementation decision gate.
4. Work on the requested tasks. A planning request does not authorize implementing its task list.
5. Verify behavior with checks proportionate to the change. Update task status, feature status, and user documentation together with the implementation.

Keep prose documentation in Markdown; task plans are the requested JSON exception. Keep implemented, planned, experimental, and excluded behavior explicit. Do not document proposed CLI examples as working commands.

All current work is v0. Avoid public API compatibility frameworks and release-version planning. Internal recipe/database format markers may be used to detect incompatibility and protect user data. Never silently discard an incompatible catalog or recipe.

## Engineering boundaries

- Framework widgets must not contain authoritative editing or catalog business logic.
- Extend commands and their query/schema documentation whenever adding user-facing operations. Verify UI/programmatic parity.
- Decode, render, import, and export work must not block the UI thread. Bound worker queues and memory; cancel stale preview work.
- Back numerical/image behavior with suitable fixtures; test source preservation and recovery, not just successful rendering.
- Pin selected dependencies. Manual license/native/asset reviews are deferred by current owner instruction; retain existing notices and do not claim an audit is complete.
- Establish reproducible setup, lint/build/package commands and agent evidence early. UI checks must inspect real rendered content with correlated state/logs, explicit capture provenance and truthful native-versus-headless results; see `docs/engineering/development.md`.
- Defer features marked future until scoped through the same process. Do not prebuild a plugin marketplace, generalized node graph, cloud service, or compatibility layer.

The maintained workspace has build commands: start with `cargo xtask check` and consult `docs/engineering/scaffold-commands.md` for build, run, smoke, audit and package commands. Probe and macOS smoke success do not establish S0 completion.
