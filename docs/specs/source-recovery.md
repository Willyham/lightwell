# Referenced originals and source recovery

Status: **M1 manual Locate agreed on 2026-09-19; owner-facing recovery defaults accepted during TASK-028; unimplemented**. The owner references existing photos, usually edits from local storage, then syncs back to an external drive. Other photographers' storage workflows require later research. Lightwell must preserve edits when originals are moved or reorganized.

## Principles and data model

An asset has a permanent catalog ID. Its path is a changeable locator. A content fingerprint verifies which bytes that locator currently names; it is not the asset's primary ID. Keep the edit recipe and history attached to the asset ID throughout moves, temporary offline states and relinking.

Store source roots separately from relative paths, plus availability state and the last verified source fingerprint. M1 records a full streamed content hash during its background single-file import. Read/hash/decode from a consistent source snapshot where possible and detect concurrent modification; do not pair a hash of one version with pixels from another. File size/mtime and filesystem IDs are fast hints, not proof that two files contain the same photo. Future imports can schedule hashing with bounded background work rather than blocking large-library startup.

Byte-identical copies can share a fingerprint without being the same catalog asset. Never merge assets or their edits merely because hashes match. A content change, including a metadata rewrite inside the file, invalidates a strict full-file fingerprint; preserving edits across such a replacement is a future explicitly reviewed workflow.

## M1: Locate missing original

1. Detect an unavailable source and retain the asset, recipe and history. Show the last known path and cached preview when present, labelled as unavailable-original content. Missing means unavailable; do not infer deletion simply because an external drive is disconnected.
2. Offer **Locate original** and allow the user to select an existing file. Do not search the whole disk or all attached drives automatically.
3. Verify the selected file against the stored full fingerprint, using bounded/cancellable I/O. A match can be renamed or reside on another volume. A mismatch is reported without replacing the source, even if filename, EXIF or dimensions look similar.
4. If the candidate already belongs to a different catalog asset, show that fact and ask for an explicit decision to use the same file; do not silently merge records. The ordinary single-asset case needs no extra ceremony beyond selecting a matching file.
5. Atomically update the locator/availability and source revision, preserve the original asset ID, recipe and edit history, and invalidate only locator-dependent state. Notify GUI/CLI/MCP clients. The preview and export resume against the verified source.

Cancellation, inaccessible candidates, mismatches and database write failures leave the prior record intact. Recheck identity if the file changes between verification and use. Export must not silently use a stale preview or unrelated file found at the old path.

Expose the same operation programmatically, with asset ID, candidate locator and an expected revision. Include proposed command schemas in the shared registry. Human and agent relinking follow the same validation, conflict and progress rules. A source-location change must not erase the geometry undo stack; keep location audit information distinct from recipe undo as needed.

M1 does not move, copy, synchronize or delete original files. The user can point Locate at the verified external-drive copy after their own sync; Lightwell changes only its catalog reference. If both copies remain, selecting the active location is explicit. Automatic local-cache/archive switching and multi-location tracking remain future work.

## Later: folder moves and automated assistance

Research how photographers use internal SSDs, removable drives, NAS, managed copies, sidecars and multiple computers before promising support. In particular, establish how they distinguish a working copy from an archive and which application performs sync.

Proposed follow-up behavior:

- Relink a root/folder with a dry-run showing matched, missing, changed and ambiguous files. Apply a verified mapping as a recoverable catalog operation.
- Search only user-selected roots, with progress/cancellation and bounded I/O. Filter candidates with cheap metadata; verify final matches with the full fingerprint. Filename alone never authorizes reassignment.
- Use filesystem change notifications or platform file references as accelerators, with reconciliation after missed events, remounts and app downtime. They are not a cross-platform identity guarantee.
- Treat an offline volume differently from a changed file. Preserve cached browsing and edit data; require the original for a full-quality export.
- Define multiple source locations and local working-copy caches only after the owner/community workflow research. Do not infer that two same-named files on different volumes are synchronized.
- Make catalog backup, original backup and sync responsibilities explicit. Relinking is not a backup strategy, and Lightwell does not become a sync engine by implication.

Sidecars may eventually help catalog portability and recovery, but add write/conflict/identity choices of their own. Keep the current catalog authoritative until that workflow is specified.

## M1 acceptance

- Import and edit a JPEG, close the app, rename/move it, reopen, Locate the same bytes, and export the original edits successfully.
- Repeat using a verified byte-identical copy on another mounted volume; asset ID and edit history remain unchanged.
- Reject a different photo with the same filename, a changed-content file, and a source modified during verification.
- Exercise a duplicate candidate already cataloged under another ID; no silent merge or edit loss occurs.
- Unplug/unmount the source, cancel verification, and simulate a failed locator commit; the catalog retains its last valid state.
- Relink through MCP while the GUI is open and observe the same recovery. Concurrent edits/location changes respect revision checks.

Native external-drive acceptance uses an explicitly chosen test directory and copies of fixtures, never the user's archive by default. VM shared-folder behavior is not proof of native removable-drive support.
