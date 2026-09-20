# M1 layers and edit history

Status: **M1 implemented and locally verified**. This is the working foundation used by M2 transforms and required before the planned tool-module and crop milestones. See [the roadmap](../design/history-first-roadmap.md) and [implementation evidence](../engineering/m1-m2-results.md).

## State model

A referenced source is read-only and identified independently of its locator. An edit layer contains a stable layer ID, effect type/format and parameters at a declared position in an ordered recipe. Each immutable recipe snapshot contains the complete stack needed to reconstruct the image. A history entry records the action that produced that snapshot.

The first proof appends single-pixel replacement layers. Two edits at the same coordinates expose order: the newer replacement wins and undo reveals the older color. Later adjustment of a layer creates a new snapshot retaining its identity; older stacks remain unchanged. A transaction may change several parameters while producing one action entry.

Import creates Original with an empty stack. Record stable asset/entry/snapshot/layer IDs; per-asset chronological sequence; action/provider identity and validated parameters; actor/timestamp/request ID; base/result revision; complete resulting stack; undo parent; and restore target when applicable. Sequence orders entries; timestamps are display information.

No full bitmap per history entry, private widget undo stack or module-owned database writes. Evaluate a snapshot from original pixels and ordered operation payloads, without replaying an evolving command-handler log.

## Commit, undo, redo and restore

A real image change atomically persists its snapshot/entry, current pointer, monotonic revision, redo state and retry result. Validate all shared and effect invariants first. Rejected actions, failed writes, no-op changes, pointer motion, Cancel and browsing add no successful edit row.

Undo navigates to the current entry's undo parent. Redo returns along the persisted redo path. Both change the concurrency revision and save navigation atomically; neither adds an Undo/Redo image row. This prevents repeated undo oscillating between navigation commands.

Restore copies any retained snapshot, including Original, into a new semantic Restore action whose undo parent is the previously current entry. It retains later actions. Restoring an equivalent current state is a no-op. A new edit/restore clears shortcut redo availability while keeping all old states available in the chronological log.

For example: Original → A → B → Restore A → C retains A and B. Undo C returns to Restore A; undo again returns to B. Undoing back to A and making D creates a new path from A while retaining B, Restore A and C for browsing. Reopening restores the complete log and current/redo navigation state.

## Read-only history preview

The history browser lists Original and bounded pages of attributed semantic actions. Mark the current committed state separately from the selected historical entry. Show explicit Previewing history, Return to current and Restore controls; keyboard and pointer selection use the same service as the API.

Selecting/rendering an entry changes no committed stack, revision, action log or redo state. Return to current displays the latest committed snapshot, even after an external edit during preview. A headless snapshot-render request does not change GUI selection unless the caller explicitly invokes session selection.

Every preview result identifies the actual source/snapshot/entry and render generation. Rapid selection supersedes obsolete work. Missing sources/providers retain history and produce explicit render limitations. Cache/memory/job bounds must hold for a long log; loading every entry or buffer to show the first page is unacceptable.

Editing while viewing history requires Return to current or explicit Restore. The later export operation captures the committed stack; exporting a historical stack requires restoring it first.

## Catalog and live API

SQLite is the initial durable storage route. Catalog reopen preserves Original, complete layers/snapshots, entry identities, current state, redo navigation and request deduplication within its documented scope. Internal format errors preserve data and report an actionable recovery path. Unapplied drafts remain session-only.

Use one application service and one catalog owner for UI and JSON/IPC clients. Expose discoverable schemas for asset/layer/state queries, pixel action, list/inspect/render/select-history/return-current/restore/undo/redo, viewport state and jobs. Expected revision and request ID guard mutations; actor attribution is shared. Reusing a request ID with different payload fails. A retry cannot duplicate an entry.

The pixel editor has x/y and RGB controls, Apply, and optional pointer picking, backed by one semantic command. Coordinates are integers in the input stage, x right/y down, after original EXIF orientation. Values are 8-bit sRGB; invalid inputs fail explicitly. Use actual source detail at 100% to inspect a changed pixel.

Draft conflicts become relevant for the crop module: Restore/undo/redo may not silently discard an active draft. Agent commits preserve drafts and mark conflict. Explicit discard/reapply resolves it against the current revision.

## Verified acceptance

1. Import → pixel A → pixel B. Verify exactly one layer/action per real change, ordered values and unchanged original bytes. Test same-pixel overrides and no-op/invalid requests.
2. Select Original and every entry through UI and API. Compare complete stack and exact lossless pixel buffers. Preview must leave committed state and revision unchanged.
3. Undo/redo, restore A, make another edit and reopen. Verify every saved state remains reconstructable with stable IDs, coherent redo/navigation and incrementing revisions.
4. Exercise an independent live client while the GUI is open, stale revisions, duplicate/different-payload request IDs, event gaps, reconnect and client/job isolation.
5. Interrupt writes and imports; test malformed/incompatible catalogs, locked or unwritable storage, changed/missing originals and unsupported effect payloads. Preserve the last valid state.
6. Browse a long history rapidly with bounded queries/jobs/memory. Correlate actual native M4 pixels, entry labels, source detail and render generation using screenshots/state/logs.

M1's exact-buffer, persistence, recovery, live-client and native M4 journey passed on 2026-09-20. M2 repeated the relevant cases with transform stacks. M3 must prove unchanged saved data through module integration, and M4 adds crop drafts, geometry snapshots and conflict cases.
