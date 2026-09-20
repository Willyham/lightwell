# History-first editor milestones

Status: S0 is accepted. M1 history and M2 transforms are implemented and locally verified; M3 modules and M4 crop remain planned.

## Sequence and scope

| Milestone | Deliverable | Completion evidence |
| --- | --- | --- |
| M1 — history foundation | **Implemented.** Non-destructive edit layers, a test pixel edit, durable history, undo/redo, restore, historical previews, catalog save/reopen, UI and live API | Passed Original → two pixel changes → preview → undo/redo → restore → new change → reopen, through UI and API, with original bytes unchanged |
| M2 — basic transforms | **Implemented.** Rotate clockwise/counterclockwise and horizontal/vertical reflection, using the existing layer/history system | Passed asymmetric fixture and pixel-edit compositions, undo/redo, preview and reopen through UI and API |
| M3 — tool modules | A small tool interface declaring actions, schemas, controls and processing; pixel editor implemented as a module | Generic UI and API discover and invoke the pixel module; saved M1/M2 documents and history still render identically |
| M4 — crop module | Lightroom-style crop workflow delivered as a tool module, including straightening and aspect constraints | Direct manipulation, numeric controls, API parity, history/reopen and composition-preserving geometry at multiple zoom/DPI settings |

Milestones run in this order. Every task file owns its own IDs starting at `TASK-001`; dependencies and task references stay inside that file. Roadmap sequencing uses named outcomes, with no cross-file task IDs or checker gates. See [M1/M2 implementation evidence](../engineering/m1-m2-results.md).

Library, tonal, RAW and external-extension work are later roadmap phases. Export, manual Locate, the MCP adapter, complete editor packaging/performance and Windows/Linux editor verification remain explicit editor follow-up work. The first milestone already requires a callable API attached to the open GUI, so later adapters do not introduce an alternate history service.

## Core model: source, layers and history

The original JPEG is a read-only asset with a stable catalog identity, mutable locator and verified fingerprint. An edit layer is a small, identified operation in an ordered recipe: a type identifier, payload format marker, parameters, input/output coordinate contract and position in the stack. “Layer” here means an edit operation; independently composited bitmap layers, opacity/blend modes and arbitrary layer reordering are not selected by this plan.

Applying an action creates a new immutable recipe snapshot and one attributed history entry atomically. The first pixel action appends a pixel-replacement layer; the next action appends another. A later explicit adjustment of an existing layer creates a new snapshot retaining that layer's stable ID. Earlier snapshots never change. A history entry describes a user/program action and points to its complete resulting stack; it is not the layer itself. A future transaction can modify several layer parameters while creating one semantic history entry.

Start with ordinary small snapshots and SQLite transactions. Do not store a full rendered bitmap per edit, replay a log of potentially changed command handlers to recover state, or treat the GUI's widgets as the authority. Rendering evaluates a saved stack from verified original pixels. Pure discrete geometry can be composed exactly; interpolated geometry must have an explicit sampling contract and may only be fused when operation order is preserved.

The implemented M1 model follows these rules. General module registration and declarative control rendering arrive in M3 after the core has been exercised with concrete operations. M1/M2 keep typed operation handlers behind the common service and stable operation identity, without building a plugin runtime in advance.

## Pixel edit as the executable proof

The test operation replaces exactly one pixel at integer `(x, y)` in the operation's input image, with top-left origin and x right/y down. Its color is an explicit three-channel 8-bit sRGB value; reject nonintegral/out-of-bounds coordinates and invalid channels. Apply EXIF orientation exactly once before the first operation. Replacing a pixel with its current evaluated value is a reported no-op and adds no entry.

Use lossless decoded buffers and synthetic fixtures to assert exact values; JPEG re-encoding is not a one-pixel correctness oracle. Two writes to the same location prove ordering: the later replacement wins, and undo exposes the earlier value. A pointer selection or explicit x/y fields and RGB controls drive the same command. Source-resolution inspection at 100%, with zoom and pan as session state, makes the affected pixel observable. A Fit preview alone cannot prove that single-pixel changes render correctly.

Each operation's coordinates refer to its input stage. A pixel edit before a rotation moves with the image; a pixel edit after it addresses the rotated dimensions. Returning to an earlier snapshot restores its dimensions and stack together. Test these cases in M2 and after module migration in M3. This avoids retroactively moving an earlier edit when a later transform is added.

## History transactions and persistence

- Import establishes Original with an empty stack. Every successful change records entry/asset IDs, per-asset sequence, action and parameters, actor, timestamp, request ID, base/result revision, complete stack snapshot and undo-parent link.
- Undo navigates to the current entry's undo parent; redo follows a persisted redo path. Both atomically increment a monotonic concurrency revision without appending oscillating Undo/Redo rows. New actions clear shortcut redo availability and retain all old entries.
- Restore copies a selected snapshot into a new Restore action, retains all later entries and undoes back to the previously current state. Restoring an equivalent current snapshot is a no-op.
- History selection and preview are read-only session operations. They change neither current recipe, revision, log nor future export input. Return to current shows the latest committed state, including a change made by another client during preview.
- SQLite is the initial persistence choice, following the existing catalog direction. Save recipes/history/current entry/redo state/request results atomically, with internal format checks and explicit failures. Drafts are not durable. Sidecars remain a separately scoped alternative; M1 does not write into the original JPEG.
- Reopen retains stable asset/layer/entry IDs and every snapshot. Missing or changed sources preserve saved edits and report a rendering limitation. A missing provider or unsupported payload is retained and reported, never silently omitted or reset.

Use one catalog owner and revision-checked commands for UI and external JSON sessions. Request IDs have a documented deduplication scope; retry returns the original result, and a reused ID with different input fails. Keep commands, log pages, event buffering and render jobs bounded. Slow storage, import/hash/decode, history queries and preview work stay off the UI thread. Rapid preview selection cancels or supersedes obsolete work, and every result carries snapshot and generation identity.

## M2 transforms

Provide quarter-turn left/right, Mirror horizontal and Flip vertical. Names specify the axis so UI/API use is unambiguous. Use exact integer mappings for these operations: no interpolation, accumulated raster edits or irreversible writes. Dimensions swap for quarter-turns. Test four rotations and double reflections as identities, plus noncommuting combinations and EXIF-mirrored sources. Every action uses the M1 transaction and history service.

## M3 module contract

A tool module supplies a stable identity and effect format, action input/result schemas, parameter units/defaults/ranges/errors, declarative control descriptions, availability/state queries, feature validation and processing implementation. The host supplies catalog access, atomic changes/history, rendering jobs and notifications. Modules cannot write catalog tables or maintain a private undo stack.

Controls describe meaning (number, color, enum, action, grouping and an optional canvas interaction adapter); the GUI chooses presentation. UI events and API requests call the same declared action handler. Registration is cheap; resources initialize when needed. Use linked modules initially. Independently loaded third-party binaries and their trust/packaging lifecycle remain later work.

Convert the pixel operation to a module without changing its durable effect identity or rewriting saved history. Migrate the M2 transform handlers behind the same interface as part of host integration, preserving the same recipes and outputs. Do not introduce a compatibility framework: for any unavoidable internal format change, use an explicit tested conversion that preserves data or stop with an actionable incompatibility result.

## M4 crop module

Recreate the familiar Lightroom crop interaction using original implementation and the recorded research. Exact proprietary rendering algorithms are not available in the sources. The module owns crop/straighten validation, fitting and rendering; the generic shell uses its controls and canvas adapter.

Include free edge/corner handles, moving the composition within the frame, thirds overlay, aspect lock and swap, Free/Original/1:1/3:2/4:3/16:9/custom ratios, numeric straightening and a drag-to-straighten guide, Apply/Cancel and reset. Option/Alt uniformly scales around the fixed center. Fine straightening is limited to ±45 degrees. Evaluate every gesture against its starting snapshot so a sweep away and back does not cumulatively shrink the composition. Preserve the chosen center/ratio where feasible and trim only as needed to avoid uncovered corners.

Define crop coordinates in the crop layer's input stage and document the normalized representation, output dimension rounding, sample centers, filter/color domain and composition with prior/later transforms before implementing it. API parameters fully reproduce a crop without pointer gestures. Keep pan distinct from crop movement (Space-drag), and support Fit, percentage and actual 100% detail on high-DPI displays. One Apply produces one history entry. Cancel and pointer motion produce none. A live agent commit preserves an active draft and marks a conflict; Discard or explicit reapply is required before overwriting current state.

## Follow-up work and limits

The editor follow-up plan owns JPEG quality-90 export, no-clobber publication/source-alias protection, metadata stripping/Keep metadata, supported ICC/output correctness, verified Locate, MCP conformance and complete editor acceptance. These requirements follow M4 and remain unimplemented. A native external-drive test needs an explicitly chosen fixture directory; no archive scan or sync is implied.

Every milestone ends with proportional automated correctness/recovery checks and a native M4 UI/API demonstration with actual rendered content tied to state/logs. Label Windows/Linux automated builds separately from deferred native desktop/GPU evidence. Numerical performance budgets remain proposals until measured and accepted. Full general layers UI, tonal tools, masks, RAW, Library, cloud services and external-loader implementation remain later scope.

Open engineering details are bounded outputs of their owning tasks: SQLite/dependency pins and data limits in M1, the module descriptor representation in M3, and crop fitting/filter tolerances in M4. Resolve each detail within its milestone's scope.
