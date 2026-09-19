# Lightwell user guide

Status: **early working scaffold on macOS**. Run `cargo xtask develop` after the [developer setup](engineering/scaffold-commands.md). The unsigned local development package opens supported JPEGs at Fit. Cross-platform acceptance is unfinished; the M1 editor below remains planned.

## What Lightwell is for

Lightwell is intended for photographers who want a fast, focused desktop library and non-destructive editor. Development starts on an M4 MacBook Pro; the first skeleton must also launch and load images on the accepted Windows/Linux targets. The later editor exposes the same functions to scripts and agents. See the [project plan](plan.md) and [feature status](features.md).

## First build: open and view

Launch Lightwell, choose **Open image**, select a supported JPEG and view it fitted to the window. Resize the window or open another image. The source stays unchanged; there are no editing tools or catalog in this build. [Skeleton scope and acceptance](specs/bootstrap.md).

Use Open image or Cmd+O on macOS. Cancelling leaves the current photo unchanged; an unsuccessful replacement shows a short error and retains the previous image. Sources are read only. Support covers 8-bit RGB/greyscale JPEG, EXIF orientation and standard sRGB; unsupported profiles are rejected. No calibrated wide-gamut display claim is made.

Developers can run repeatable synthetic fixture scenarios and inspect logs, state and renderer screenshots with the [working commands](engineering/scaffold-commands.md).

## Subsequent M1 editor workflow

1. Create or open a local catalog. Import references photos already on disk.
2. Import a JPEG and select it for editing. Supported orientation and color metadata are interpreted before display.
3. Open Crop and adjust its sides/corners freely or choose a locked aspect ratio. Hold Option to scale proportionally about the center. Straightening preserves your composition with only necessary trimming. Use pan, a zoom percentage or Fit to inspect the image. Apply saves a geometry recipe; Cancel discards the draft.
4. Rotate or flip as needed. Undo/redo changes the recipe without altering the original photo.
5. Close and reopen the catalog to continue from saved edits.
6. Export a new JPEG to share or use in other software. Optional metadata is stripped by default; enable Keep metadata to preserve supported source information. Color and orientation remain correct in either mode. The catalog retains the editable recipe.
7. If the original was moved or renamed, use Locate original to select and verify the file. This reconnects the existing photo and edits rather than reimporting it as a new asset.

Crop resizing/straightening, metadata defaults and zoom/pan, percentage and Fit controls are agreed. Their detailed contract and remaining implementation details are in the [one-image specification](specs/single-image.md).

## Originals, catalogs and exports

The **original** is the imported source file. The **catalog** records its identity, location and editing instructions. The **preview cache** holds disposable renderings that can be rebuilt. An **export** is a new image with the selected edits rendered into pixels.

A catalog does not back up your original photos. Moving a source outside Lightwell may make it unavailable until it can be relinked. The initial app will retain its edits and offer manual Locate, checking that the selected file contains the expected bytes. It will not silently substitute a same-named photo. Folder-level recovery and automatic location detection are planned later. Catalog backup and photo backup are separate operations; consistent catalog snapshots are part of the storage design.

## Scripts and agents

The subsequent M1 editor includes JSON commands and local MCP for catalog operations, import/Locate, reading image/edit state, geometry changes, history, previews and export. An agent can act while the GUI is open, and its edits appear in the same photo and undo history. Concurrent changes are checked against revisions so an agent cannot silently overwrite a newer human edit. S0's development smoke controls precede this full agent API.

These operations use the same validation and edit service as the interface. Automation does not require a built-in chat assistant, a subscription, or a particular model provider. The [agent contract](design/architecture.md#agent-contract) will become the implementation reference; concrete usage examples will be added only when the commands exist.

## Planned limitations of the first editor

The first supported input is JPEG. RAW, PNG, tonal controls, large library browsing and installable plugins come in later scoped work. Untagged JPEGs are assumed to be sRGB; unsupported color modes or unusable profiles are reported. Initial RAW targets are Nikon Z6 and Fujifilm X100VI; support will be claimed by tested recording mode, not solely by filename extension or an upstream camera list.

These editor limitations apply to M1; S0 has the smaller image-loading scope above. The current repository includes a maintained load-only viewer, developer checks, synthetic fixtures and isolated comparison probes.
