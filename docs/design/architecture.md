# Editor architecture

Status: M1/M2 implemented, following the owner's [history-first sequence](history-first-roadmap.md). The Rust/Iced desktop now uses the working core/catalog/API boundaries below. M3 modules and M4 crop remain planned.

## Ownership and stages

The UI and external clients use one application service. That service owns asset state, recipe/history transactions and bounded work. Rendering consumes immutable snapshots. M1/M2 use concrete typed handlers; M3 introduces a module registry and declarative controls after those handlers have proved the core.

| Boundary | Responsibility |
| --- | --- |
| Desktop shell | Layout, focus, pointer capture and adaptation to semantic commands; display authoritative state |
| Core model/service | Asset/layer/snapshot identities, validation of shared invariants, atomic commits, revisions, retries, history navigation |
| Catalog | Read-only source references and verified fingerprints; durable layers/snapshots/history/current/redo/request results |
| Renderer and scheduler | Evaluate an immutable ordered stack from verified originals; bounded memory, cancellation and generation identity |
| Tool modules, from M3 | Action schemas, parameter/control descriptions, feature validation and processing via host services |
| JSON/IPC and later MCP | Transport to the same catalog owner and operation discovery; no alternate persistence or edit logic |

Start with the existing small Rust workspace. Add boundaries when there is real code to own them; crates and dynamically loaded binaries are separate choices.

## Sources, layers and snapshots

Import references a supported JPEG and creates a stable asset plus Original. Path/root are mutable locators, while a full fingerprint verifies the original bytes. Keep source reads consistent across hashing and decoding. Same-filesystem aliases resolve to one asset; identical copies at different paths are not implicitly merged. Missing/changed sources preserve edits and return explicit rendering limitations. Manual Locate is retained in editor follow-ups.

An edit layer is an identified, typed operation with parameters and an explicit input/output stage contract. A recipe is the ordered stack of those layers. A history entry identifies a semantic action and stores its complete resulting immutable recipe. Adding a layer and selecting a history step are different operations. Updating a layer produces a new snapshot; it never mutates an old entry.

M1 proves this with single-pixel replacement. M2 adds exact transformations; both are implemented and retain meaningful operation order. A pixel set before a rotation travels with that image, while a pixel set after it addresses the rotated stage. M4 crop will use its own input-stage coordinates. Optimize processing only where equivalence preserves this order.

## Persistence and recovery

Use local SQLite for the first catalog implementation. Keep originals and disposable pixel caches outside the database. Persist current state, an action entry/snapshot, monotonic revision, redo navigation and request result atomically. Short transactions and explicit internal format checks protect v0 data. A failed write must preserve the prior durable state and never display Saved.

Entries are the only stored copy of a stack at catalog format 2; a format 1 catalog converts once on open. Versions are named references to entries and lineage follows the stored undo-parent column; see [versions and lineage](versions-and-lineage.md). Retain every history entry. Undo/redo navigate saved snapshots and increment revision without adding inverse actions. Restore creates a new action copying a selected snapshot. New changes clear shortcut redo but retain old branches for inspection/restore. Reopening must recover the same layer/entry IDs, current snapshot and navigation state. See [the history contract](../specs/edit-history.md).

Backups require a consistent SQLite snapshot; copying an active database file is not a documented backup method. Sidecars, managed originals and synchronization remain later product work. Unknown payloads/provider effects are retained and reported; never silently reset an incompatible catalog or drop unavailable edits.

## Rendering, color and memory

For M1 use the viewer's supported JPEG subset, orient once to upright pixels and give every buffer explicit color meaning. Test pixel replacement uses 8-bit sRGB channels and integer coordinates on its input stage. Tiny lossless expected buffers establish exact correctness. Do not use JPEG re-encoding to prove only one pixel changed.

Discrete M2 transforms use exact mappings. M4 supplies a numerical proof for crop/straighten, source coverage, inverse sample positions and linear-light interpolation before its processor is implemented. Extended ICC conversion and export metadata are explicit follow-ups. The earlier proposed wide-gamut float working space is not a prerequisite for the pixel/history foundation.

Bound active/pending jobs and estimated bytes, cancel superseded preview work and attach source/snapshot/generation identity to results. Fit uses a bounded preview; 100% resolves source detail in physical framebuffer pixels. Enlarged Fit content is a temporary loading state. Never allocate an image per history row. Paginate history and retain only bounded display/render data.

## Agent contract

M1 exposes catalog/import/state, pixel edits, history/undo/redo/restore, preview selection/render, session view state and jobs through one typed service and external JSON sessions. Use bounded local same-user IPC to a single catalog owner while the GUI is open; permit headless ownership when absent. Protect against competing owners and stale revisions.

Assets are discoverable through `catalog.list`, so a client that did not import can find asset identities. `render.sample` evaluates one pixel of a saved entry without rasterizing. `version.create`, `version.delete`, `version.list` and `history.lineage` expose named states and the undo-parent chain. The owner holds each registered client's session and reports a session revision with every session-returning response. Commands have schemas, units, ranges, defaults and structured results/errors. Durable actions require expected revision and request ID, with a documented deduplication scope. Reconnect reads a fresh state/sequence rather than blindly replaying mutations. Client disconnection does not cancel another client's jobs. Diagnostics stay off protocol stdout.

Preview and session state are not image history. An external commit updates current state while a selected historical snapshot stays selected. Crop drafts introduced in M4 remain intact and conflicted after external commits until explicit resolution. MCP later adapts the same operation registry.

## Modules and extension path

M3 makes concrete actions discoverable as modules with semantic controls and processors. The pixel and transform effects keep their durable identity so existing catalogs/history render unchanged. The generic shell chooses how to draw controls and supplies gesture adapters; modules may not write catalog tables or invent private undo.

Linked built-ins with lazy resources are sufficient for the four milestones. Later external loading remains required and will be scoped around a selected use case, measured costs, resource/trust limits and a separately authored loaded module. See [the module contract](modules-and-api.md). No marketplace, stable public ABI or general node graph is required.
