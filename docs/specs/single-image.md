# One-image editor and crop-module contract

Status: **planned**. The editor has four milestones: history foundation, transforms, tool modules and crop. Export, Locate and MCP are editor follow-ups. See [the current roadmap](../design/history-first-roadmap.md). Application implementation remains on hold.

## Staged editor scope

M1 establishes by-reference JPEG import into a local catalog, an ordered non-destructive layer stack, a pixel-change proof, persistent history/undo/redo/restore, historical previews and reopen. A focused UI and live external JSON API use the same service. The [history specification](edit-history.md) owns this first acceptance journey.

M2 adds exact rotations and both reflection axes with history and persistence. M3 adds the tool interface, API/action schemas and semantic controls, migrating the pixel and transform tools. M4 implements the crop/straighten tool through that interface.

The UI uses one workspace with compact collapsible panels, a central photo and visible history. Cmd/Ctrl+O imports; Cmd/Ctrl+Z and Shift+Cmd/Ctrl+Z undo/redo. Visible controls have keyboard access and predictable focus. No full grid or placeholder future tools are required.

## Viewport and coordinates

Fit, editable zoom percentage, 100% and pan are explicit session operations available through UI and API. Define 100% against source pixels and physical framebuffer pixels; do not conflate it with logical UI pixels on Retina. A magnified Fit texture may appear while source detail loads only with a clear loading state. Bound work and reject stale results.

Image-edit coordinates are independent of viewport zoom and DPI. Each layer acts in its input image stage; a later transform must not reinterpret an earlier pixel edit. For crop UI, map the canvas through the view transform into the crop layer's input space. Space-drag pans. Ordinary drag inside the crop repositions the composition/crop relative to the image, with the crop-frame behavior established in the interaction proof.

## Geometry contract

Apply source EXIF orientation once before layer evaluation. Basic quarter-turns/reflections are exact integer mappings. M4 crop combines fine straightening and crop into a layer with explicit input/output geometry; user transforms before or after it retain their sequence.

Within the crop layer, clockwise fine straightening rotates its input around the image center and the crop is axis-aligned in the transformed bounding box. Normalize x/y/width/height to that bounding box, top-left origin, x right/y down. Values are finite, extents positive and all crop corners lie inside valid transformed source coverage. Pixel aspect ratio accounts for bounding-box dimensions. Do not apply the earlier single-global-geometry order over an ordered layer stack.

M4's initial proof defines output-size rounding, pixel-center sampling, inverse mapping, interpolation filter/color domain and tolerances before implementation. The candidate convention is to floor positive extents to whole pixels and sample through the inverse transform, rejecting extents below one pixel; locked-ratio rounding must have a documented pixel tolerance. Cardinal transforms remain exact. Fusion of geometry is allowed only when it preserves intervening effect order.

## Crop module controls

Include Free, Original, 1:1, 3:2, 4:3, 16:9 and custom ratios; locked ratio orientation can swap. Original means the upright original's ratio adjusted for preceding quarter-turn orientation. Expose angle (−45° to +45°), a drag-to-straighten guide, Apply, Cancel and reset. Use quarter-turn controls for larger rotation. A later quarter-turn carries the visible crop and swaps its displayed ratio orientation; reflections carry the off-center composition with the image.

Provide free edge/corner handles, crop movement and a thirds overlay. In Free mode a side moves independently and corners can change width/height. Holding Option (Alt on Windows/Linux) applies a common scale factor about the fixed center, preserving the current aspect ratio even in Free mode. A locked ratio stays locked. Clamp the common factor at the first source boundary; do not shift the center or independently stretch an axis.

Straightening preserves composition as closely as possible, retaining the selected center/ratio where feasible and trimming only as needed to avoid empty corners. Define the fitting objective and tie-breaks in the M4 proof with off-center and near-edge fixtures. Each pointer update evaluates against the gesture's starting crop/angle snapshot so dragging away and back does not cumulatively shrink the crop.

A draft is transient. Apply/Enter commits one semantic action and one new layer-stack snapshot; Cancel/Escape discards it. Adjusting an existing crop layer preserves its ID in the new snapshot. Reset restores that tool's neutral state through the history service, while a whole-recipe reset is an explicit separate action. No pointer event commits history or destructively resamples a saved image.

Every parameter and action is exposed in the module's API; gesture simulation is unnecessary. The shell renders semantic controls and supplies a canvas adapter, while the module owns geometry validation and processing.

## History and conflicts

Every committed pixel, transform or crop action uses the core history service. Preview shows a retained snapshot without changing current state, revision or future export input. Restore appends a new action; all historical states survive. Reopen preserves layer IDs, snapshots, current history and redo navigation.

Agent commits during a human draft retain that draft and mark it conflicted. Offer Discard or explicit Reapply against the latest revision, then revalidate; stale revisions never overwrite silently. Restore/undo/redo do not discard drafts implicitly. During history preview, changes require explicit Return to current or Restore first. Current-state notifications do not retarget the selected historical entry.

## Retained export and recovery contract

Editor follow-ups evaluate the committed snapshot and write a new JPEG at source-scale crop dimensions, quality 90 by default. There are no resize presets, watermarks or batch export in this slice. Export freezes the revision and effective metadata option so later edits/settings cannot affect an in-flight job.

Convert to sRGB and embed a valid profile. Normalize/omit EXIF orientation, regenerate dimensions and remove obsolete thumbnails. Keep metadata starts off: omit optional EXIF/IPTC/XMP capture, camera, creator/copyright and GPS fields. With it on, retain the explicitly supported valid descriptive/capture/GPS fields while correcting structural information. A metadata reader/writer and exact field/container policy must be proven; private manufacturer metadata round-tripping is not promised.

Use a native destination picker with a suggested source-stem plus `-edited.jpg`. Reject all existing files and source aliases, including symlinks/hardlinks. Write/flush a temporary file in the destination directory and publish without replacement. Failure/cancellation removes only temporary output. Independently inspect output pixels, tags, profile, orientation and dimensions in both metadata modes.

Manual Locate verifies the expected source fingerprint, updates its locator atomically and preserves asset/layer/history identity. Missing/changed originals retain edits and report their limitations. See [source recovery](source-recovery.md). The MCP adapter exposes the common service and schemas with a real protocol/conformance journey; it adds no separate feature logic.

## Crop and complete-editor acceptance

1. Use the M1 saved-history demo as the baseline; repeat it with M2 transforms and module-based handlers before crop testing.
2. Test every crop handle, movement, aspect/angle control and Option scaling at Fit, numeric zoom and 100% on supported display scales, with distinct pan/crop gestures.
3. Sweep angle away/back, use off-center subjects and near-boundary crops, and compose before/after pixel/transform effects. Check coverage, output size, sampling tolerance and source detail.
4. Apply/Cancel, reset, undo/redo, preview Original/intermediate states, restore, make a new edit and reopen. Verify exactly one action per commit and retention of every snapshot.
5. Use live UI/API clients with stale revisions, draft conflicts, restore/preview/reconnect, malformed input and failed writes. Compare complete stacks and decoded outputs.
6. Correlate native M4 actual rendered content with state/logs, entry IDs, revisions and render generation; measure queues/memory/latency and verify original hashes.
7. In editor follow-ups, add verified Locate, both metadata export modes, MCP interoperability and complete package acceptance. Native Windows/Linux verification remains separately deferred.

RAW, PNG, tonal controls, multi-image library and externally loaded modules remain later scope. A Lightroom-style interaction reference does not imply Adobe rendering compatibility.
