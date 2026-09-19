# M1 edit history

Status: **planned M1 core requirement; not implemented**. The owner clarified on 2026-09-19 that history is a log of all editing actions, shared by every tool, with preview and restore at any point. Undo/redo alone is insufficient. This supplements the [one-image editor](single-image.md), [accepted M1 decisions](../design/m1-decisions.md) and [module host](../design/modules-and-api.md).

## Coverage review

The existing M1 tasks cover geometry, persistence, export, source recovery and live automation. TASK-008 already requires transactional durable history; TASK-007/011 establish shared schemas/services; TASK-013 offers undo/redo; TASK-016 verifies parity. They do not explicitly require a browsable log, arbitrary-entry preview/restore or retention of states outside the redo path. The architecture's previous “new edit replaces the redo branch” wording could allow historical edits to disappear.

M1 is sufficient for the agreed basic **geometry editor** once these gaps are closed. Tonal editing remains M3; this clarification does not add exposure, white balance, masks or RAW to M1. Those later tools must use the same history contract when introduced.

## Core contract

- Each asset has a persistent Original entry representing its imported default recipe, followed by ordered entries for every successful committed image change. Crop Apply (including straightening and automatic crop fitting), rotate, flip and Reset Geometry all use the host transaction service. Future tools inherit this rule. Human and agent actions share one log.
- Store a stable entry ID, asset ID, chronological sequence, semantic operation/provider ID and validated parameters, human-readable action summary, actor, timestamp, request ID, base/result revision and sufficient immutable recipe state to reconstruct the complete result. Record restoration/undo relationships explicitly. Sequence defines ordering; timestamps are display information.
- The log describes actions; the resulting recipe describes rendering. Revisiting a point renders its complete saved state from the unchanged original through the common pipeline. Do not reverse pixel operations or depend on replaying old commands through possibly changed tool code. Ordinary SQLite tables with complete small recipe snapshots are sufficient for M1; a generalized event-sourcing framework is not required.
- Atomically persist the entry, resulting recipe/current-state reference, monotonic revision and retry result. Failure preserves the previous durable state and log; a retried request must not create a duplicate action within the documented deduplication scope. History navigation never decreases the concurrency revision.
- Every committed image change goes through this service, including undo/redo navigation, programmatic multi-parameter edits and future module tools. Modules and widgets cannot maintain authoritative private history. Cancelled drafts, rejected operations and no-op changes add no successful edit entry. Crop Apply is one action, not one entry per pointer event.
- Zoom/pan/Fit, selection, panel state, history browsing and preview are session operations. Import establishes Original; Locate updates source location while retaining the same asset/history. Export and export metadata settings do not alter the image recipe. These operations remain programmable but do not masquerade as image edits.
- No implicit history pruning in M1. A new edit may invalidate the shortcut redo path, but must not erase prior entries or their saved states. History and shortcut navigation state survive catalog reopen. Missing/changed sources or unavailable providers preserve entries and return explicit preview/render limitations; never substitute different source pixels or silently omit effects.

## Preview and restore

The history panel lists action summaries and attribution, distinguishes the current committed state from the selected historical entry, and includes Original. Pointer and keyboard selection can inspect any retained entry. Show an explicit historical-preview label, a Return to current action and a separate Restore action. Preview changes neither the current recipe nor the revision, log or saved state. Closing preview returns to the latest committed state.

Preview requests identify the asset and entry plus size/view requirements; results identify the actual entry, source identity and render generation. Rapid selection cancels or supersedes obsolete work so an old result cannot replace the selected entry. Paginate log queries and bound preview queues/cache/memory; keep database work and rendering off the UI thread. Do not store a full pixel buffer per entry or load the entire log to show its first page.

Restore selects the full saved recipe at an entry, including Original, through the same validated host mutation path. It requires the current expected revision and a request ID; it is itself undoable and persisted. The target entry remains inspectable. A restore to the already-current recipe is a no-op with an explicit result.

**Restore policy accepted by the owner on 2026-09-20:** append “Restore to [entry]” as a new action, retaining every later action. For example, Original → Crop → Flip → Straighten followed by restoring Crop produces a new Restore-to-Crop entry; Flip and Straighten remain available. Product TASK-070 records the accepted choice; TASK-064 verifies it before implementation.

Undo/redo operate on complete semantic actions using saved recipes and host-owned navigation state. Each new edit/restore entry links to the previously current entry as its undo parent; a restore additionally identifies the historical entry whose recipe it copies. Undo moves the current reference to its undo parent, and redo returns along the persisted redo path. These navigation transactions increment the concurrency revision and persist current/redo state atomically; they do not append image-edit entries that would make repeated undo oscillate. The chronological log and every saved entry remain intact.

Undo after Restore returns to the state immediately before Restore; redo reapplies it. A new edit or restore clears shortcut redo availability but retains those entries for browsing and restoration. For example, after undoing back to Crop and making a new Flip, the new Flip undoes to Crop, while the earlier Flip/Straighten/Restore entries remain in the chronological log. Reopen preserves both the full log and current/redo navigation state. The panel marks the current entry separately from the selected preview; its action list does not duplicate rows merely for browsing or undo/redo.

While previewing history, edit controls require Return to current or Restore before applying a new tool action. Export continues to capture the committed state; the UI explicitly identifies that state while a historical preview is visible. Exporting a historical state in M1 requires restoring it first. Unapplied crop drafts remain preserved when browsing; Restore and undo/redo return a draft-conflict result until the user explicitly discards or resolves the draft. No action silently discards it.

An agent commit during historical preview updates the current-state indicator while the selected immutable entry stays selected. Return to current shows the latest state. Restore with a stale revision fails without mutation. After reconnect, query the current revision and selected entry/session state; never blindly retry against a new revision. Historical preview cannot be confused with an unapplied crop draft.

## Programmatic contract

The shared registry must expose bounded history listing, entry inspection, historical preview, end-preview/return-to-current, restore, undo/redo and history/session state. Schemas describe entry IDs, pagination/order, actor/action information, target/current revisions, selected preview identity, capabilities and structured missing-entry/source/provider/draft/conflict errors. These are required operation families, not implemented command names.

UI, JSON and live IPC/MCP use the same implementation. A headless caller can inspect and render any entry without changing the GUI selection; showing that historical preview in the GUI is an explicit session operation. Changes emit revision/history notifications. Requesting a preview never turns into an implicit restore. Built-in and later external tools provide semantic action metadata and use the host's persistence and state-reconstruction contract.

## Acceptance and task ownership

| Outcome | Task ownership |
| --- | --- |
| Entry/snapshot/revision types and complete operation schemas | TASK-007 |
| Atomic log, snapshot retention, undo/redo navigation and recovery | TASK-008 |
| Common rendering of an immutable historical recipe | TASK-009 |
| List/inspect/preview/restore service, concurrency and session behavior | TASK-011 |
| Browsable history panel, preview/restore and keyboard interaction | TASK-071 |
| JSON and live IPC/MCP exposure | TASK-014/020/021 |
| Shell integration, reopen and conflicts | TASK-015 |
| End-to-end parity, native evidence and documentation | TASK-016/017/018; TASK-019 portability |
| Accepted restore/navigation policy and implementation gate | Completed product TASK-070, verified by TASK-064 |

1. Import → crop → flip → rotate → straighten → reset. Verify exactly one semantic entry per committed action, correct attribution and stable Original/entry IDs; Cancel, pointer motion, zoom and failed/no-op actions add none.
2. Select Original and every intermediate point. Compare complete recipes and rendered geometry with expected fixtures, independent of later edits. Preview must leave committed recipe, revision, log and export input unchanged. Return to current must work after an agent has committed.
3. Restore an intermediate point and Original, then undo/redo, make another edit and reopen. Verify every previous entry remains addressable and reconstructable, shortcut navigation is coherent, restored results match the selected snapshot and original bytes are unchanged. Apply the resolved restore policy explicitly in assertions.
4. Repeat list/inspect/preview/restore through UI, JSON, headless and live MCP. Test stale revision, duplicate restore request, active crop draft, agent commit during preview, missing entry, wrong-asset entry and reconnect.
5. Interrupt a commit and inject a failed write; reopen with recipe, revision, log and undo/redo state consistent. Relink a moved original and revisit old entries; unavailable source/provider cases retain state and report the limitation.
6. Browse a generated long history with paginated queries and rapid preview selection. Record M4 latency/memory evidence, check bounded work and confirm stale results cannot display as the selected entry. Correlate real UI captures with entry IDs, revisions and render state. A database-only test does not establish history UI completion.
