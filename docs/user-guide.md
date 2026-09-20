# Lightwell user guide

What works today: opening a JPEG, pixel-proof edits, exact transforms delivered as tool modules with generated controls, persistent history and the JSON API, verified on macOS. Crop, export, Locate and MCP are planned; see [feature status](features.md).

Basic exposure, tone, white balance and color controls, plus a histogram and clipping inspector, have a [proposed design](design/basic-and-histogram.md). They are not available yet.

## Start the editor

After [developer setup](engineering/development.md), start an optimized build with a catalog and an optional JPEG:

```sh
cargo xtask develop --catalog /path/to/catalog.sqlite --open /path/to/photo.jpg
```

Omit `--catalog` to use the platform configuration directory. `--data-root DIRECTORY` isolates config, cache and log paths. `cargo xtask develop --debug` is an unoptimized build for debugging and is unsuitable for timing.

For agent-driven API or rendered checks on macOS, add `--background` to keep the editor from taking desktop focus. Use a separate test catalog or `--evidence-dir NEW_DIR`; background evidence runs capture the editor and exit automatically. Smoke and diagnostic harnesses use background launches by default on macOS. Launch normally for keyboard, mouse and native-dialog interaction.

Open references an existing supported sRGB or greyscale JPEG without copying or modifying it. Cmd+O on macOS and Ctrl+O elsewhere opens the picker. EXIF orientation is applied once before any edit. The catalog stores stable identities, the source fingerprint, ordered operations and history. It is not a backup of the original photo.

## Edit and inspect

The tool panel is generated from the registered tool modules: each module declares its fields and buttons, and the desktop only lays them out. A field shows its unit and, while its text is outside the declared range, the message naming that range; a button is enabled only when the current state is editable and every value it needs is valid. Tab and Shift+Tab move between fields, and Enter in a field runs its action. A control kind this build cannot draw is shown as an explicit unsupported-control message rather than hidden, and an unavailable module is listed with its reason.

Pixel proof accepts integer x/y coordinates and RGB values from 0 to 255. Coordinates use the operation's input image with a top-left origin, x right and y down. Clicking the photo at Fit or any zoom fills X and Y with the pixel under the pointer without committing anything. Apply pixel creates one layer and one attributed history action when the resulting pixel changes. Invalid, out-of-bounds and same-value requests add nothing.

Exact transforms are Rotate left, Rotate right, Mirror horizontal and Flip vertical. Quarter-turns swap dimensions. Order is preserved: a pixel edit before a rotation moves with the image, and one made afterwards addresses the rotated dimensions.

Fit shows the whole image. Enter a percentage from 10 to 1600 and choose Set, or choose 100%. On a high-DPI display, 100% maps one source pixel to one physical framebuffer pixel. Both scroll axes pan content larger than the viewport.

## History

History lists Original and every committed action with sequence and actor. The filled marker is current; selecting another row previews that immutable snapshot without changing current state or revision. Return to current leaves the preview; Restore appends a new action containing the selected recipe. Editing is disabled during a historical preview.

Versions name the displayed state so you can find it again among many entries. Type a name and choose Save; each version is listed with its entry number, selecting one previews it, and Restore this state brings it back as a new action. Deleting a version removes only the name. History rows marked branch were undone and replaced by later edits; they remain available for preview, restore and versions.

Undo and Redo navigate saved states without appending rows. Cmd+Z / Ctrl+Z and Shift+Cmd+Z / Shift+Ctrl+Z invoke the same service as the buttons. A new edit clears shortcut redo while every older entry remains available for preview or Restore. Layers, history, navigation state and stable IDs survive reopening the catalog.

Missing or changed sources keep their catalog data and report why rendering is unavailable. Incompatible catalogs or operation payloads fail explicitly; Lightwell never silently resets or drops them.

## JSON automation

The headless owner reads one JSON request per line and writes one response per line. Diagnostics stay off stdout:

```sh
printf '%s\n' \
  '{"id":"schema","method":"schema.list","params":{}}' \
  '{"id":"import","method":"catalog.import","params":{"path":"/path/to/photo.jpg"}}' \
  | target/release/lightwell-json --catalog /path/to/catalog.sqlite
```

A request has `id`, `method` and `params`. A success carries the matching `id`, an event `sequence` and `result`; a failure carries a structured `error`. Start with `schema.list` for the authoritative method list and `catalog.list` for the referenced assets. Edit actions are generated from the registered tool modules: `module.list` returns every module with its effects, actions, parameter descriptors (kind, range, unit, default) and semantic controls, and each action `<id>` is callable as `edit.<id>` with its parameters as top-level fields beside `asset_id` and `mutation`. Today that is `edit.set-pixel` (`x`, `y`, `rgb`) and `edit.transform` (`transform`). Parameters are checked against the descriptors before the module sees them, so every client gets the same structured validation error. `version.create`, `version.list`, `version.delete` and `history.lineage` cover named states and the undo-parent chain. Mutations require `asset_id` and a `mutation` object:

```json
{"id":"rotate","method":"edit.transform","params":{"asset_id":"asset-…","mutation":{"expected_revision":0,"request_id":"rotate-1","actor":"my-client"},"transform":"rotate-right"}}
```

While the desktop owns a catalog it creates `CATALOG.live-session.json` beside it, recording a `127.0.0.1` address and a token. That session accepts the same newline-delimited requests with the token as the request's top-level `token`. The file is owner-readable on Unix and removed on orderly shutdown. Up to eight clients are accepted; requests are limited to 1 MiB and retained events to 256. Use `events.since`, and refresh with `asset.state` if it reports a gap.

A catalog has one owner. Starting the headless command against a catalog open in the GUI fails instead of creating competing state. Request IDs deduplicate retries; reusing one with different input fails.
