# Lightwell v0 project plan

Status: **S0 accepted; M1 history and M2 transforms implemented and locally verified; M3/M4 planned**. The owner accepted S0 on 2026-09-20 using native M4 and earlier automated portable evidence. M1/M2 have exact-buffer, persistence, API and native M4 evidence. Fresh hosted verification and deferred Windows/Linux desktop, accessibility and license reviews remain unfinished follow-ups. See [working commands](engineering/scaffold-commands.md) and [M1/M2 results](engineering/m1-m2-results.md).

## Next four milestones

The milestone sequence proves non-destructive state and history before adding increasingly complex tools. The [detailed design](design/history-first-roadmap.md) defines behavior, data contracts and acceptance.

| Milestone | Working result |
| --- | --- |
| M1 — history foundation | **Implemented.** Referenced JPEG, ordered edit layers, a pixel-change proof, persistent action history, undo/redo, restore, preview/select history, catalog reopen, desktop UI and live external API |
| M2 — transforms | **Implemented.** Exact rotate-left/right, horizontal mirror and vertical flip through the same layers/history, UI and API |
| M3 — tool modules | Shared module interface for action schemas, semantic controls, validation and processing; pixel and transform tools use it |
| M4 — crop module | Lightroom-style crop and straighten interaction as a module, with aspect controls, Apply/Cancel and UI/API parity |

Every milestone begins with the working result of the previous one and ends with a native M4 demonstration plus proportional automated correctness/recovery checks. Each task file starts at TASK-001 with local dependencies. Milestones are ordered here rather than coupled through IDs.

The implemented core uses SQLite for atomic layer/snapshot/history persistence. Original files stay read-only. A history entry saves the complete layer stack produced by an action; preview reads a saved snapshot; restore appends an action retaining all later history.

## Editor follow-ups

A dedicated editor follow-up plan covers JPEG export (quality 90, no overwrites, metadata stripped by default with Keep metadata), color/metadata proof, manual Locate, a standards-compliant MCP adapter and complete native packaging/performance/portability verification. These capabilities follow M4 and remain unimplemented.

M1 already includes a discoverable external JSON API and local ownership/IPC while the GUI is open. UI and agents share revisions and history from the outset. The later MCP adapter exposes the same operations. Source identity and missing/changed-original handling begin with the M1 catalog; verified manual relinking follows in the recovery slice.

## Product constraints

- Build a non-destructive photo editor for macOS, Windows and Linux, targeting the owner's M4 MacBook Pro first.
- Preserve original bytes; treat paths as changeable locators and asset identity as stable. Do not store authoritative edits in previews.
- Keep a small UI-independent core for recipe/history transactions, invariants and bounded services. Modules own tool semantics and processing through those services.
- Expose every operation through structured actions and state when its UI arrives, including future tools and module lifecycle.
- Keep decode, hashing, persistence, rendering and export off the UI thread; bound memory, queues and cancellation.
- Use open-source project code/extensions. GPL-3.0-or-later is selected; existing notices and automated dependency policy remain, and deferred reviews are not passing results.
- All work is v0. Internal format markers protect data; a public compatibility framework, plugin marketplace, cloud service and generalized processing graph are outside the current scope.

## Editor interaction direction

Use one focused workspace with compact collapsible controls, a central photo and a visible action history. Numeric zoom, Fit, 100% source detail and pan are session state, independent of edits. Every tool uses one shared history service. Crop uses free side/corner handles, a composition overlay, aspect lock/swap and straightening with Apply/Cancel. Option/Alt scales proportionally about the fixed center; straighten gestures preserve composition without cumulative trimming.

A layer means an ordered edit operation in a recipe. Pixel painting, transforms and crop may add or adjust layer data while history retains immutable previous stacks. General bitmap compositing, opacity/blending and a layer-reordering interface need their own later scope.

## Later roadmap

The relative priority of these later phases still needs owner input.

| Phase | Intended scope |
| --- | --- |
| Useful small library | Multi-image import, virtualized browsing, filtering/tagging/collections, lazy shoot subsets, multi-selection/stacking and broader source recovery |
| Tonal editing and PNG | Exposure, white balance, contrast and related controls, with explicit numerical/color contracts |
| Trustworthy RAW | Nikon Z6 and Fujifilm X100VI tested by actual recording mode; benchmark established decoders before custom work |
| Richer tools | Texture, clarity, dehaze and separately scoped masks/clone/healing |
| External modules | Measured activation costs and a separately authored module loaded through documented host APIs; linked built-ins do not eliminate this requirement |

Map, Book, Slideshow, Print, Web and Publish Services are excluded. Accounts, cloud sync, built-in AI chat, generative editing and a marketplace are not needed to prove the editor.

## Evidence and planning

Rust/Iced is the selected S0 stack. Reuse the maintained workspace and [developer tooling](engineering/development.md). The [feature matrix](features.md) distinguishes the implemented viewer from planned editor work. [Architecture](design/architecture.md), [history](specs/edit-history.md), [modules](design/modules-and-api.md), [crop](specs/single-image.md) and [source recovery](specs/source-recovery.md) describe the contracts.

Use [the active task index](../tasks/README.md) and [planning conventions](task-planning.md). [Owner decisions](decisions.md) record accepted requirements and open questions. [Lightroom research](research/lightroom/README.md) and [technical research](research/technical-options.md) are context, not evidence that matching features have been implemented.
