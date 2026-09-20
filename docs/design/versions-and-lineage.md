# Versions, lineage and the history graph

Status: **implemented and locally verified on 2026-09-20**. This note records what the history graph already is, adds named versions and lineage queries on top of it, and settles the catalog storage question those features raised. The owner chose the name "version" for the Lightroom-style saved state.

## The graph that already existed

Every history entry stores its complete immutable stack and an `undo_parent`. Undo and redo move the current pointer; a new edit after undo appends with the undone-to entry as its parent; nothing is ever deleted. That is a commit graph: an entry is a commit, its stack is the tree, `undo_parent` is the parent pointer, the asset's current entry is HEAD and the per-asset `sequence` is the reflog. Restore is a forward-moving revert that copies an old stack into a new entry.

Lightroom truncates later history when you edit from an earlier state, and its snapshots exist to survive that truncation. Lightwell never truncates, so every state a Lightroom snapshot could bring back is already reachable through `history.list` and `history.restore`. What was missing was a way to name one entry among hundreds, and a way to see the chain rather than the chronological list.

## Vocabulary

- **Snapshot** keeps its existing internal meaning: the immutable stack one action produced. It is not a user feature.
- **Version** is the user-facing name for a named reference to one retained entry. It is a tag, not a branch.
- **Lineage** is the undo-parent chain from an entry back towards Original.

Branches in the Lightroom sense, virtual copies with their own current pointer, are not part of this work. They would need a second head per asset and are left for a later library decision.

## Versions

A version is a row in `versions` with the asset, a name, the entry it names, the actor and a creation time. Names are unique per asset ignoring case, trimmed, one to sixty-four printable characters. Creating a version does not touch the recipe, revision or history; it emits an event so other clients refresh their lists. Creating the same name on the same entry is a no-op; on a different entry it is a conflict. Deleting a version removes only the name and is a no-op when absent, so retries are safe without request identifiers. The named entry stays in history and can still be previewed and restored after its version is deleted.

Restoring a version is the existing `history.restore` on the version's entry. There is no separate restore path to keep the operation set small.

## Lineage

`history.lineage` walks `undo_parent` from an entry (default current) newest first, returning entry id, sequence, action and parent per step, at most one hundred steps per call with `next_entry_id` to continue. It reads the new `undo_parent_id` column rather than parsing entry JSON, so the desktop can afford it after every change. The desktop marks loaded entries that are not on the current lineage as branches; when the chain was truncated it marks nothing at or below the oldest returned step, because it cannot know.

## Storage decision: catalog format 2

Format 1 wrote each stack three times: inside the entry JSON, in a `snapshots` row and as `snapshot_layers` rows. Only the entry JSON was ever read. The alternative was to turn the layer table into a content-addressed object store, which would deduplicate payloads and index layers by effect id. That is speculative for v0 payloads of a few dozen bytes, so format 2 drops both tables and keeps the entry JSON authoritative. When M3 needs to find snapshots that use a provider, a scan of entry JSON is sufficient at v0 catalog sizes, and an index table can be added with that feature.

Format 2 also adds `undo_parent_id` on entries and the `versions` table. Opening a format 1 catalog converts it in one transaction: verify that every entry's snapshot row exists, drop the entry immutability trigger to backfill the parent column, recreate the trigger, drop the two tables, create `versions` and set the format marker. A catalog whose snapshot rows are inconsistent is left at format 1 with an incompatibility error rather than converted. Format 0 creates format 2 directly; any other marker is refused.

## Sessions live with the owner

The same change moved client sessions into the catalog owner, keyed by a registered client id. Previously the desktop threaded a copy of its session through every task and wrote the copy back on completion, so a slow read-only refresh could overwrite a newer zoom or selection. Now only the owner mutates sessions, every session-returning response carries a session revision, and the desktop adopts a response only when its revision is at least the one it holds. Pan goes through `view.set` like zoom, with one request in flight and only the newest pending position.

## API

| Method | Effect |
| --- | --- |
| `version.create` | Name an entry (default current); no-op when the name already names it |
| `version.delete` | Remove a name; the entry remains |
| `version.list` | Versions in creation order with their entry sequence |
| `history.lineage` | Undo-parent chain from an entry, paged |

The method table in the core carries each method's schema description, mutation flag and handler, and a test checks that the published schema, the event rule and dispatch agree.

## Acceptance

- Format 1 catalogs convert once; inconsistent ones are refused unchanged; converted catalogs reopen at format 2 with only the expected tables.
- Versions survive reopen, reject empty, oversized and control-character names, treat case-insensitive duplicates as conflicts, restore through the ordinary restore path and keep their entry after deletion.
- Lineage skips abandoned branches, pages with a continuation id and rejects unknown assets.
- An independent JSON client creates a version, undoes, lists versions and reads a one-step lineage in one session.
- Desktop: stale session responses are not adopted, pan coalesces to one in-flight request, and a refresh replaces or merges history and marks branches. `cargo xtask check` passes.
