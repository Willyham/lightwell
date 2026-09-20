# Product decisions

S0 is accepted. M1 history and M2 transforms are implemented and locally verified. The remaining delivery sequence is M3 tool modules and M4 crop; export, Locate, MCP and complete editor verification follow. See the [project plan](plan.md), [M1/M2 results](engineering/m1-m2-results.md) and [accepted editor behavior](design/m1-decisions.md).

## Product and platform

- Build a fast, non-destructive desktop photo editor for professional and prosumer collections on macOS, Windows and Linux.
- Target the owner's M4 MacBook Pro first. The measured host is an M4 Pro with 14 CPU cores, 20 GPU cores and 48 GB unified memory, running macOS 26.5.2.
- Initial engineering baselines are macOS 14+ arm64, Windows 11 24H2+ x64 and Ubuntu 24.04 x64 with GNOME Wayland and X11. These are intended targets, not proof that minimum-version or native Windows/Linux checks pass.
- Use unsigned development packages. Locally built software may be built and run for development/testing.
- Manual Windows/Linux desktop and manual license/native/asset reviews are deferred. Preserve existing notices and automated dependency checks. Deferred reviews are not passing results.
- Project code is GPL-3.0-or-later. Project/runtime dependencies and extensions should be open source and compatible with that license; proprietary hosted services are not development prerequisites. Native SDK/toolchain requirements are documented separately.
- Everything is v0. Do not add public compatibility frameworks, release-version planning, cloud/accounts, a marketplace or generalized processing graphs.
- Initial RAW targets are the original Nikon Z6 and Fujifilm X100VI. Recording modes and private sample coverage need verification. Benchmark established libraries before proposing a custom decoder.
- Library/Develop are familiarity references. Map, Book, Slideshow, Print, Web and Publish Services are outside the selected scope.

The [platform matrix](engineering/platforms.md) distinguishes target choices from verified runtime evidence. The [dependency review](engineering/dependency-review.md) states current review limits and temporary advisory exceptions.

## Implemented S0

The accepted S0 skeleton opens one supported JPEG at Fit with a dark shell, Open image action, loading/error feedback, automatic EXIF orientation and retention of the previous photo when replacement fails. Its deterministic evidence mode remains separate from the normal M1/M2 editor, which now owns a catalog and edit API. Export and MCP are still absent.

The supported subset is 8-bit RGB/greyscale JPEG: untagged sRGB, supported standard tagged sRGB and explicit errors for unsupported/malformed profiles. Broad ICC conversion and professional monitor calibration are not established.

Rust/Iced is selected. Project development tooling uses Rust xtask for local and CI checks. S0 acceptance rests on native M4 evidence plus the recorded green portable build/package run. Fresh hosted verification of the closure snapshot remains unfinished; S0 acceptance does not imply that check or native Windows/Linux acceptance passed.

## Editing and storage

Original files remain read-only. Import references existing files, with stable asset IDs, verified fingerprints and changeable locators. SQLite is the initial local catalog direction. Folder relinking, sidecars, catalog portability, backups and sync need separately scoped workflow decisions.

M1 implements ordered non-destructive edit operations (“layers”), immutable complete recipe snapshots and durable attributed history. The pixel-change proof exercises the model before more complex tools. General bitmap compositing, blend modes and arbitrary layer reordering are not selected.

Every committed image action enters the shared history. Undo/redo navigates saved entries without appending oscillating navigation rows. Preview/select is read-only. Restore appends an action and preserves all later entries; a new edit clears shortcut redo availability but retains historical access. Committed state survives restart; drafts do not.

Use one focused workspace with collapsible controls, a centered photo, visible history, Fit, numeric zoom and true 100% inspection. No full library grid is required for the editor milestones. Geometry, crop, export, recovery and human-agent behavior are specified in [accepted editor behavior](design/m1-decisions.md), [history](specs/edit-history.md), [editor](specs/single-image.md) and [source recovery](specs/source-recovery.md).

## Programmable operations and modules

Every application operation must be programmable, including all bundled/external tools, masks, clone strokes, settings and module lifecycle when those features arrive. Expose structured actions, discoverable schemas, state and errors with each feature; GUI gestures cannot be its only interface.

The core owns recipe transactions, shared history/undo, invariants and bounded services. Tool modules own their parameters, validation, controls and algorithms through those APIs. The same command service handles human and agent actions.

Live JSON/IPC access is implemented in M1 while the GUI is open. Agents and the GUI share revisions/history. Active human draft conflict behavior arrives with M4 because M1/M2 have no drafts. MCP is an editor follow-up using the same operations.

M3 introduces linked tool modules. Cheap registration and lazy resource initialization are the engineering direction, not a measured performance claim. Measure before splitting basic functionality into separately loaded binaries. Actual external module loading remains required later; missing or disabled providers must never silently erase edits or produce incomplete exports.

The first external use case, optional boundaries, package/runtime format, trust/isolation and UI contribution details remain open. See [module/API design](design/modules-and-api.md).

## Owner workflow priorities

The owner commonly edits local files and syncs them back to an external drive. Moved-original recovery is a core concern. Research broader photographer workflows before selecting storage or sync behavior.

| Priority | Direction |
| --- | --- |
| Speed and unused-tool bloat | Measure startup, loading, idle and first-use costs; keep the core small and initialize resources lazily |
| Filtering, tagging and collections | Design a consistent retrieval model from concrete workflows |
| Full catalog with a lazy shoot/subsection | Keep catalog scale independent of decoded pixels; scope queries and views to the working set |
| Multi-selection and stacking | Define selection ranges, stack identity, representative images and batch semantics in later library work |
| Bracket/panorama identification | Research detection/grouping separately; HDR/panorama merging is not selected |
| Confusing export controls | One clear JPEG export path, quality 90, no overwrites and explicit metadata behavior |

These are priorities and problem statements, not authorization to implement every possible solution.

## Open product questions

- How should catalog backup, portability, sidecars, folder relinking and external-drive sync work?
- Should RAW precede a useful small library or tonal tools? Which actual camera modes and camera-JPEG/film-simulation matching matter most?
- What is the first external module the owner would use, and what enablement/recovery behavior does it need?
- Which measured workloads and responsiveness budgets should become acceptance requirements?

Resolve consequential choices with the owner. Keep proposals distinct from accepted requirements, and update the affected current specification when an answer is accepted. Routine engineering details may be chosen within the agreed scope. Use proportionate Markdown/task planning as described in [planning guidance](task-planning.md).
