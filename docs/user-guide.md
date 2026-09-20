# Lightwell user guide

Status: **M1 history and M2 exact transforms are working and locally verified on macOS**. S0 is owner-accepted. Native Windows/Linux desktop checks, M3 modules, M4 crop and the editor follow-ups remain open.

## Start the editor

After [developer setup](engineering/scaffold-commands.md), start an optimized build with a catalog and optional JPEG:

```sh
cargo xtask develop --catalog /path/to/catalog.sqlite --open /path/to/photo.jpg
```

Omit `--catalog` to use Lightwell's platform configuration directory. Use `--data-root DIRECTORY` to isolate development config/cache/log paths. `cargo xtask develop --debug` selects an unoptimized build and is unsuitable for timing.

Open references an existing supported sRGB/greyscale JPEG; it does not copy or modify the file. Cmd+O on macOS and Ctrl+O elsewhere opens the picker. Automatic EXIF orientation is applied once before edits. The catalog stores stable identities, the source fingerprint, ordered operations and history; it is not a backup of the original photo.

## Edit and inspect

Pixel proof accepts integer x/y coordinates and RGB values from 0–255. Coordinates use the operation's input image, with top-left origin, x right and y down. Apply pixel creates one layer and one attributed history action when the resulting pixel changes. Invalid, out-of-bounds and same-value requests do not add an edit.

Exact transforms provide Rotate left, Rotate right, Mirror horizontal and Flip vertical. Quarter-turns swap dimensions. Transform and pixel order is preserved: a pixel edit before a rotation moves with the image; one made afterward addresses the rotated dimensions.

Fit shows the whole image. Enter a percentage from 10–1600 and choose Set, or choose 100%. On a high-DPI display, 100% maps one source pixel to one physical framebuffer pixel rather than one logical UI point. Both scroll axes pan content when it is larger than the viewport.

## History

History shows Original and committed actions with sequence and actor. The filled marker is current; selecting another row previews that immutable snapshot without changing current state or revision. Use Return to current to leave preview, or Restore to append a new action containing the selected recipe. Editing is disabled during historical preview.

For an unchanged active source, Lightwell reuses one decoded pixel allocation while browsing history. Exact rotate/reflect stacks are composed before rendering instead of rewriting the image once per history layer. This is transparent to recipes and does not weaken changed-source detection.

Undo and Redo navigate saved states without appending artificial history rows. Cmd+Z / Ctrl+Z and Shift+Cmd+Z / Shift+Ctrl+Z invoke the same service as the buttons. A new edit clears shortcut redo while every older entry remains available for preview or Restore. Layers, history, current/redo navigation and stable IDs survive reopening the catalog.

Missing or changed sources preserve their catalog data and report why rendering is unavailable. Incompatible catalogs or operation payloads fail explicitly; Lightwell does not silently reset or drop them.

## JSON automation

The headless owner reads one JSON request per line and writes one response per line. Diagnostics stay off stdout:

```sh
printf '%s\n' \
  '{"id":"schema","method":"schema.list","params":{}}' \
  '{"id":"import","method":"catalog.import","params":{"path":"/path/to/photo.jpg"}}' \
  | target/release/lightwell-json --catalog /path/to/catalog.sqlite
```

A request has `id`, `method` and `params`. Successful responses contain the matching `id`, an event `sequence` and `result`; failures contain a structured `error`. Start with `schema.list` for the authoritative method list and `catalog.list` for the referenced assets. Mutations require `asset_id` and:

```json
{
  "expected_revision": 0,
  "request_id": "unique-retry-key",
  "actor": "my-client"
}
```

For example, after obtaining an asset ID and current revision:

```json
{"id":"rotate","method":"edit.transform","params":{"asset_id":"asset-…","mutation":{"expected_revision":0,"request_id":"rotate-1","actor":"my-client"},"transform":"rotate-right"}}
```

While the desktop owns a catalog it creates `CATALOG.live-session.json` beside it. That mode accepts the same newline-delimited requests over the recorded `127.0.0.1` address, with the recorded token added as the request's top-level `token`. The file is owner-readable on Unix and is removed during orderly shutdown. Up to eight simultaneous clients are accepted; requests are limited to 1 MiB and retained events to 256. Use `events.since`; if it reports a gap, refresh with `asset.state`.

The catalog has one owner. Starting the headless command against a catalog already open in the GUI fails instead of creating competing state. Request IDs deduplicate retries in the catalog; reusing one with different input fails.

## Planned crop, export and recovery

M3 will introduce the generic tool-module contract. M4 will add crop with free handles, ratios, straightening, thirds overlay, Apply/Cancel and Option/Alt scaling around a fixed center. These controls are not present yet.

The later JPEG export uses quality 90, suggests `-edited.jpg`, never overwrites an existing destination or original, strips optional metadata by default and offers Keep metadata for supported fields. Manual Locate will reconnect a moved original only after verifying its content. MCP will adapt the existing service; the working JSON API is not an MCP server.

See [feature status](features.md), [M1/M2 evidence](engineering/m1-m2-results.md) and [active task plans](../tasks/README.md) for exact scope and verification limits.
