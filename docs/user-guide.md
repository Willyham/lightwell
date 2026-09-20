# Lightwell user guide

Status: **working S0 viewer; editor features planned and implementation on hold**. The owner accepted S0 on native M4 evidence and earlier portable checks. Fresh hosted verification and native Windows/Linux desktop follow-ups remain open.

Run `cargo xtask develop` after [developer setup](engineering/scaffold-commands.md). It uses an optimized build by default; `--debug` selects an unoptimized debugging build. The unsigned local macOS package opens supported sRGB/greyscale JPEGs at Fit.

## Current viewer

Use Open or Cmd+O on macOS / Ctrl+O elsewhere. The viewer applies supported JPEG orientation, fits the whole image and recomputes Fit when resized. A failed replacement preserves the previous photo and displays an error. Tab focuses Open; Return/Space activates it. The viewer does not yet save edit data or expose editing tools.

## Planned milestones

| Milestone | What becomes usable |
| --- | --- |
| M1 — history | Open a local catalog, reference a JPEG, change one pixel with a test tool, browse/preview/restore edit history, undo/redo and reopen saved edits |
| M2 — transforms | Rotate left/right, mirror horizontally and flip vertically, each recorded in the same history |
| M3 — modules | Tools declare their actions and controls; the pixel and transform tools use the shared module interface |
| M4 — crop | Familiar crop/straighten controls, free handles, aspect locks, Apply/Cancel and full history support |

M1 includes a live API for using the same operations while the GUI is open. Export, manual Locate and MCP remain planned editor follow-ups after these four milestones.

## Edits, layers and history

An original is a read-only source photo. A layer is a saved editing operation, such as a pixel change or rotation. The catalog stores an ordered layer stack and a complete snapshot for each history action. Editing never rewrites the source JPEG.

History will show Original and every committed action, with human/agent attribution. Selecting a row previews its saved image without changing the current edit. Return to current exits that preview. Restore creates a new action using the selected state while preserving everything that came later. Undo/redo navigate saved states; making a new edit clears shortcut redo availability but retains earlier entries in history.

Committed changes and navigation will survive reopening the catalog. Unapplied drafts will not. A catalog does not back up original photos. Missing or changed sources retain their edits and report why rendering is unavailable.

## Planned crop and export behavior

Crop will offer free side/corner handles, aspect ratios, straightening and a thirds overlay. Option/Alt scales proportionally around the fixed center. Straightening preserves composition with necessary trimming, and Apply commits one action; Cancel discards the draft. Fit, numeric percentages, 100% and pan are independent viewport settings.

An agent edit during a human crop draft preserves the draft and shows a conflict. Explicit discard or reapply resolves it. Every operation has an API counterpart.

The later JPEG export uses quality 90, a suggested `-edited.jpg` filename and a new destination. Optional metadata is stripped by default, with Keep metadata available for supported descriptive fields. Color/orientation/dimensions remain correct in both modes. Existing files and originals are never overwritten.

Manual Locate will reconnect a moved original only after verifying its content, preserving the same asset, layers and history. Folder search, automatic sync and sidecars remain later work.

See [the roadmap](plan.md), [feature status](features.md) and [active task plans](../tasks/README.md) for scope and progress.
