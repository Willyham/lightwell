# Working on Lightwell

## Product constraints

- Build a non-destructive photo editor for macOS, Windows, and Linux, serving professional and prosumer collections.
- Target the owner's M4 MacBook Pro first. Keep Windows/Linux portability visible; distinguish VM functional checks from native GPU performance evidence.
- The first end-to-end build is S0: a cross-platform skeleton that opens and displays a JPEG at Fit. No editing tools, catalog, export or production MCP are required in S0. Current owner override: S0 is accepted on recorded native M4 and earlier portable evidence; fresh hosted closure-snapshot verification and manual Windows/Linux desktop checks remain unfinished follow-ups. M1 adds the editor and is currently paused by owner instruction.
- Prefer open-source-only project code and extensions. The owner selected GPL-3.0-or-later and the repository license is applied. Manual license reviews are deferred for now; selection does not imply an audit is complete.
- Initial RAW compatibility targets are Nikon Z6 and Fujifilm X100VI. Benchmark established libraries before proposing a new decoder; custom work requires a documented support or performance gap.
- Preserve original files. Keep edits as data and use a common command layer for UI and programmatic operations.
- Every application operation must be programmable, including every bundled/external edit tool, future masks/clone strokes, settings and module lifecycle. Add discoverable schemas and state access with the feature; GUI gestures cannot be its only interface.
- Keep a small shell/module host: the core owns recipe transactions, history/undo, shared invariants and bounded services; feature modules own their validation and processing through those APIs. Prefer useful optional modules and lazy resource initialization; require measurements before splitting basic core functionality into separately loaded binaries. External module loading remains required later, even if built-ins stay linked. See `docs/design/modules-and-api.md`; do not add a plugin runtime to S0/M1 or silently drop edits when a provider is disabled/missing.
- Follow the owner's history-first sequence: M1 implements ordered edit layers, a pixel-change proof, persistent history/undo/redo/restore/preview, catalog save/reopen, UI and live JSON/IPC API; M2 adds exact rotate/flip/mirror; M3 introduces tool/action/control modules; M4 implements the Lightroom-style crop module. See `docs/design/history-first-roadmap.md`.
- Import references existing files; stable asset IDs and edits survive locator changes. Manual Locate, JPEG export/color/metadata, MCP and full-editor verification remain explicit editor follow-ups after those four milestones. Folder relinking and sync research follow later.
- Crop controls in M4 include free handles, proportional Option scaling around the fixed center and composition-preserving straightening. M1 already needs zoom/pan with percentages, Fit and actual 100% detail for the pixel proof. Export strips optional metadata by default with a Keep metadata setting and correct output color/geometry.
- Treat responsiveness, bounded memory, image correctness, and agent access as architectural requirements.
- Prefer a small, beautiful core with sensible defaults and deliberate extension points. Use Lightroom Library/Develop as a familiarity reference, not a feature checklist.
- Consult the owner on consequential product tradeoffs. Record recommendations as proposals until decided; do not turn unanswered interview questions into accepted decisions.

## Proportionate planning workflow

1. Read the product documents relevant to the requested work. When working on an existing milestone or planned task, also read `docs/task-planning.md`, the relevant specification, and its active plan from `tasks/README.md`.
2. Before implementing a substantial new feature, milestone, architectural change, migration, or other coordinated multi-step body of work, create or update a Markdown design/plan describing behavior, scope, constraints, acceptance criteria, and unresolved decisions.
3. Use the Create Tasks skill when the user asks for a task plan or when substantial work needs dependency-aware decomposition. Routine research, reviews, diagnostics, documentation maintenance and small contained changes do not require task-plan changes unless requested or already tracked. Each task file has a local ID space starting at TASK-001, ordered by dependencies; do not reference tasks in another file. Keep milestone sequencing in Markdown by named outcomes. Keep IDs and statuses stable during routine updates. Keep documentation and task context focused on current decisions, behavior, evidence and outstanding work; keep only current plans and omit planning-change logs. Validate every changed graph.
4. Work only on the requested scope. A planning request does not authorize implementing its task list.
5. Verify behavior with checks proportionate to the change. For work explicitly tracked by a task plan, update its status together with the implementation. Update feature status and user documentation when behavior or documented scope changes.

Keep prose documentation in Markdown; task plans are the JSON exception when a task plan is warranted. Keep implemented, planned, experimental, and excluded behavior explicit. Do not document proposed CLI examples as working commands.

All current work is v0. Avoid public API compatibility frameworks and release-version planning. Internal recipe/database format markers may be used to detect incompatibility and protect user data. Never silently discard an incompatible catalog or recipe.

## Engineering boundaries

- Framework widgets must not contain authoritative editing or catalog business logic.
- Extend commands and their query/schema documentation whenever adding user-facing operations. Verify UI/programmatic parity.
- Decode, render, import, and export work must not block the UI thread. Bound worker queues and memory; cancel stale preview work.
- Back numerical/image behavior with suitable fixtures; test source preservation and recovery, not just successful rendering.
- Pin selected dependencies. Manual license/native/asset reviews are deferred by current owner instruction; retain existing notices and do not claim an audit is complete.
- Establish reproducible setup, lint/build/package commands and agent evidence early. UI checks must inspect real rendered content with correlated state/logs, explicit capture provenance and truthful native-versus-headless results; see `docs/engineering/development.md`.
- Defer features marked future until scoped through the same process. Do not prebuild a plugin marketplace, generalized node graph, cloud service, or compatibility layer.

The maintained workspace has build commands: start with `cargo xtask check` and consult `docs/engineering/scaffold-commands.md` for build, run, smoke, audit and package commands. S0 is accepted; individual follow-ups still require their own verification evidence.
