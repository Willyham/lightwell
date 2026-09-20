# Crop, export and conflicts

Status: the M4 crop module is planned; export, Locate and MCP are editor follow-ups. History behavior comes from [layers and history](edit-history.md).

## Viewport and coordinates

Fit, an editable zoom percentage, 100% and pan are session operations available through UI and API. 100% means one source pixel to one physical framebuffer pixel, never a logical UI pixel. A magnified Fit texture may appear while source detail loads only with a clear loading state. Image-edit coordinates are independent of zoom and DPI: each layer acts in its input stage, and a later transform never reinterprets an earlier edit. For crop UI, map the canvas through the view transform into the crop layer's input space. Space-drag pans; ordinary drag inside the crop moves the composition.

## Geometry contract

EXIF orientation is applied once before layer evaluation. Quarter-turns and reflections are exact integer mappings. The crop layer combines fine straightening and crop with explicit input and output geometry; transforms before or after it keep their sequence.

Within the crop layer, fine straightening rotates the input around the image center and the crop is axis-aligned in the transformed bounding box. Normalize x, y, width and height to that box with a top-left origin; values are finite, extents positive, and every corner lies inside valid transformed source coverage. Before implementation, M4 defines output-size rounding, pixel-center sampling, inverse mapping, interpolation filter and color domain, and tolerances. Candidate convention: floor positive extents to whole pixels, sample through the inverse transform, reject extents below one pixel, and give locked-ratio rounding a documented tolerance. Geometry may be fused only when intervening effect order is preserved.

## Crop module

Ratios: Free, Original, 1:1, 3:2, 4:3, 16:9 and custom, with a locked ratio able to swap orientation. Original means the upright original's ratio adjusted for preceding quarter-turns. Angle from −45° to +45°, a drag-to-straighten guide, Apply, Cancel and reset; quarter-turn controls handle larger rotation. A later quarter-turn carries the visible crop and swaps its ratio orientation; reflections carry an off-center composition with the image.

Free edge and corner handles, crop movement and a thirds overlay. In Free mode a side moves independently and a corner changes width and height. Holding Option (Alt on Windows/Linux) applies one scale factor about the fixed center, preserving the current ratio even in Free mode, clamped at the first source boundary without shifting the center or stretching an axis.

Straightening preserves composition as closely as possible, keeping the selected center and ratio where feasible and trimming only enough to avoid empty corners. M4 defines the fitting objective and tie-breaks with off-center and near-edge fixtures. Each pointer update evaluates against the gesture's starting crop and angle so dragging away and back never cumulatively shrinks the crop.

A draft is transient. Apply or Enter commits one semantic action and one new snapshot; Cancel or Escape discards. Adjusting an existing crop layer keeps its ID. Reset restores the tool's neutral state through the history service; whole-recipe reset is a separate explicit action. No pointer event commits history or resamples a saved image. Every parameter and action is in the module's API; gesture simulation is unnecessary.

## Conflicts

An agent commit during a human draft keeps the draft and marks it conflicted, offering Discard or explicit Reapply against the latest revision followed by revalidation. Stale revisions never overwrite silently. Restore, undo and redo do not discard drafts implicitly. During history preview, changes require Return to current or Restore first. Current-state notifications do not retarget the selected historical entry.

## Export (follow-up)

Export evaluates the committed snapshot and writes a new JPEG at source-scale crop dimensions, quality 90 by default, freezing the revision and metadata option so later changes cannot affect an in-flight job. No resize presets, watermarks or batch export. Output is sRGB with a valid embedded profile, normalized orientation, regenerated dimensions and no obsolete thumbnail. Keep metadata starts off, omitting optional EXIF, IPTC and XMP capture, camera, creator, copyright and GPS fields; when on, the explicitly supported valid fields are retained with corrected structural information. The reader and writer, exact field policy and tolerances must be proven; private manufacturer metadata round-tripping is not promised.

A native destination picker suggests the source stem plus `-edited.jpg`. Reject every existing file and source alias, including symlinks and hardlinks. Write and flush a temporary file in the destination directory and publish without replacement; failure or cancellation removes only temporary output. Inspect output pixels, tags, profile, orientation and dimensions independently in both metadata modes.

Manual Locate and the MCP adapter are specified in [source recovery](source-recovery.md) and [architecture](../design/architecture.md#agent-contract); MCP adds no separate feature logic.

## Acceptance

1. Repeat the M1/M2 saved-history journey through module-based handlers before crop testing.
2. Exercise every handle, movement, ratio and angle control and Option scaling at Fit, numeric zoom and 100% on supported display scales, with distinct pan and crop gestures.
3. Sweep angle away and back, use off-center subjects and near-boundary crops, and compose with pixel and transform effects before and after. Check coverage, output size, sampling tolerance and source detail.
4. Apply, Cancel, reset, undo, redo, preview, restore, edit again and reopen: exactly one action per commit and every snapshot retained.
5. Live UI and API clients with stale revisions, draft conflicts, restore and preview during reconnect, malformed input and failed writes; compare complete stacks and decoded output.
6. Native M4 rendered content correlated with state, logs, entry IDs, revisions and render generation, plus queue, memory and latency measurements and unchanged original hashes.
7. Follow-ups add verified Locate, both export metadata modes, MCP interoperability and complete package acceptance.

RAW, PNG, tonal controls, a multi-image library and externally loaded modules remain later scope. A Lightroom-style interaction reference does not imply Adobe rendering compatibility.
