# Layers, history and exact transforms

Status: implemented (M1 and M2) and verified on the M4 Mac. This is the foundation every later tool uses.

## State model

A referenced source is read-only and identified independently of its locator. An edit layer has a stable layer ID, effect type and format, and parameters at a position in an ordered recipe. Each immutable recipe snapshot holds the complete stack needed to reconstruct the image. A history entry records the action that produced a snapshot: stable asset, entry, snapshot and layer IDs; per-asset sequence; action and provider identity with validated parameters; actor, timestamp and request ID; base and result revision; the complete resulting stack; undo parent; restore target when applicable. Sequence orders entries; timestamps are display information.

Import creates Original with an empty stack. Later adjustment of a layer creates a new snapshot that retains the layer's identity; older stacks never change. A transaction may change several parameters while producing one action entry.

No full bitmap per history entry, no private widget undo stack, no module-owned database writes. A snapshot is evaluated from original pixels and its ordered payloads, never by replaying an evolving command log.

## Commit, undo, redo and restore

A real image change atomically persists its snapshot and entry, the current pointer, a monotonic revision, redo state and the retry result, after validating all shared and effect invariants. Rejected actions, failed writes, no-op changes, pointer motion, Cancel and browsing add no row.

Undo moves to the current entry's undo parent; redo follows the persisted redo path. Both change the concurrency revision and save navigation atomically without adding rows. Restore copies any retained snapshot, including Original, into a new Restore action whose undo parent is the previously current entry, keeping all later actions. Restoring an equivalent current state is a no-op. A new edit or restore clears shortcut redo while every state stays in the chronological log.

Example: Original → A → B → Restore A → C keeps A and B. Undo C returns to Restore A; undo again returns to B. Undoing back to A and making D starts a new path from A while B, Restore A and C remain browsable. Reopening restores the complete log and navigation state.

## Read-only preview

The history browser lists Original and bounded pages of attributed actions, marking the current committed state separately from the selected entry, with explicit Previewing, Return to current and Restore controls. Selecting or rendering an entry changes no committed stack, revision, log or redo state. Return to current shows the latest committed snapshot even after an external edit during preview. A headless snapshot render does not change GUI selection unless the caller invokes session selection. Every preview result identifies its source, snapshot, entry and render generation; rapid selection supersedes obsolete work. Editing while previewing requires Return to current or Restore first. Export later captures the committed stack; exporting a historical stack requires restoring it.

## Pixel proof

The pixel editor exposes x/y and RGB controls, Apply and optional pointer picking, backed by one semantic command. Coordinates are integers in the input stage after EXIF orientation, x right and y down; values are 8-bit sRGB; invalid input fails explicitly. Replacing a pixel with its current value is a reported no-op. Exact lossless buffers on synthetic fixtures are the correctness oracle; JPEG re-encoding is not. Two writes to the same location prove ordering: the later wins and undo exposes the earlier.

## Exact transforms

Coordinates use a top-left origin, x right, y down. Every layer addresses its input stage.

| Operation | Input to output mapping | Output size |
| --- | --- | --- |
| Rotate right | `(x, y) → (h - 1 - y, x)` | `h × w` |
| Rotate left | `(x, y) → (y, w - 1 - x)` | `h × w` |
| Mirror horizontal | `(x, y) → (w - 1 - x, y)` | `w × h` |
| Flip vertical | `(x, y) → (x, h - 1 - y)` | `w × h` |

Mappings are integer-exact with no interpolation, accumulated raster edits or irreversible writes. Four matching quarter-turns and two matching reflections are identities. Order stays observable: a pixel edit before a transform moves with the image, one after it uses the transformed dimensions. Tests cover identities, non-commuting combinations, EXIF-mirrored sources and every interleaving with pixel edits.

## Versions and lineage

A version names one retained entry per asset, unique ignoring case, without changing the recipe, revision or log. Creating, listing and deleting versions are programmable and emit events; restoring a version is the ordinary restore of its entry, and deleting a version never removes history. Lineage walks undo parents from any entry newest first in bounded pages, and the desktop marks loaded entries off the current lineage as branches. Client sessions are held by the catalog owner per registered client and carry a revision; a client adopts only responses at least as new as the session it holds. Design and storage decision: [versions and lineage](../design/versions-and-lineage.md).

## Catalog and live API

Catalog reopen preserves Original, all layers and snapshots, entry identities, current state, redo navigation and request deduplication. Internal format errors preserve data and report a recovery path. Drafts are session-only.

One application service and one catalog owner serve UI and JSON/IPC clients. Discoverable schemas cover asset, layer and state queries, every module action as a generated `edit.<action>` method with its parameter descriptors, module descriptors, history list, inspect, render, select, return-to-current, restore, undo and redo, viewport state, pixel sampling and jobs. Expected revision and request ID guard mutations; a retry returns the original result and a reused ID with a different payload fails. Draft conflicts come from the crop module: Restore, undo and redo do not silently discard an active draft, and agent commits preserve drafts and mark them conflicted.

## Acceptance

1. Import → pixel A → pixel B: exactly one layer and action per real change, ordered values, unchanged original bytes, same-pixel overrides and rejected no-op or invalid requests.
2. Select Original and every entry through UI and API, comparing complete stacks and exact buffers, with committed state and revision unchanged.
3. Undo, redo, restore, edit again and reopen: every state reconstructable with stable IDs, coherent navigation and incrementing revisions.
4. An independent live client while the GUI is open: stale revisions, duplicate and conflicting request IDs, event gaps, reconnect and client/job isolation.
5. Interrupted writes and imports, malformed or incompatible catalogs, locked storage, changed or missing originals and unsupported payloads all preserve the last valid state.
6. Rapid browsing of a long history within bounded queries, jobs and memory, with native M4 pixels correlated to entry labels and render generation.

The golden journey in `fixtures/history/m2-journey.json`, recorded before the module integration, reopens with identical identities, stacks, navigation and pixels; M4 adds crop drafts, geometry snapshots and conflict cases on top of that same history mechanism.
