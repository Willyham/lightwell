# First working editor: one JPEG

Status: **subsequent M1 editor; detailed specification remains draft; nothing implemented**. The owner confirmed geometry edits, undo, reopen and export on 2026-09-19, then inserted the smaller [S0 image-loading skeleton](bootstrap.md) as the first build. Dependencies: S0, M1 decision gate TASK-064 and geometry/color proof TASK-004. Agreed behavior is labelled below; remaining defaults are proposals, not claims of exact Lightroom behavior.

## Scope and interface

Create/open a local catalog, reference an existing JPEG, display it, rotate in 90-degree steps, flip horizontally, crop, straighten, undo/redo, reopen the saved result, and export a new JPEG. Include manual Locate for moved originals, plus JSON commands and local MCP with live GUI/agent coordination for all durable actions and state queries. Full grid browsing, RAW, PNG, tonal controls, ratings, plugins and cloud services are outside M1. The agreed recovery scope is detailed in the [source recovery design](source-recovery.md).

The default window has an unobtrusive catalog/import area, central image, and compact geometry panel. Show actual loading, saved, unsaved, missing-source and error states. No empty Lightroom modules or disabled future sliders. The view should look intentional with just one image.

At first import, show the photo fit to the available canvas with correct orientation. **M1 includes zoom/pan, percentage settings and Fit**, as confirmed by the owner. Provide an editable percentage control, a 100% inspection option and Fit; validate usable continuous zoom inputs during the UI trial. Percentage limits and gesture bindings are proposed implementation details, not reasons to defer the agreed controls. Native file dialogs and standard platform undo/redo shortcuts are expected.

Keep viewport zoom/pan separate from the crop recipe and exported dimensions. Crop handles must map correctly at every supported zoom and display scale. Define 100% against image and physical framebuffer pixels in the M1 geometry/display trial, with a clear numeric readout; do not conflate it with OS logical UI pixels on Retina displays. Fit recomputes for the available canvas. Panning moves the viewport, while dragging inside the crop edits its position; an explicit gesture/modifier must distinguish them. Expose zoom percentage, Fit and pan through the session API for agents as well as the UI.

At 100%, resolve the visible region from source-resolution pixels. A magnified Fit preview may appear temporarily while detail loads, with an explicit loading state, but must not be presented as finished full-detail inspection. Keep requests and memory bounded; M1 need not introduce a persistent tile pyramid to achieve this. Validate with fine-detail fixtures and during pan/zoom changes so obsolete detail cannot replace the current view.

## Import and persistence

- The user chooses a catalog location and one source JPEG. Import validates format, dimensions, profile and readability off the UI thread, then commits an asset and default recipe atomically. Reimporting the same filesystem asset returns the existing asset; byte-identical files at different paths are not silently merged.
- A failed import creates no usable asset or misleading success state. A partially generated cache can be discarded. Report unsupported formats, missing files, malformed data, resource limits, and storage failures distinctly.
- The original is opened for reading only. All editing metadata stays in the catalog; M1 writes no source-adjacent sidecar.
- Committed edits are autosaved. A save failure must not show “Saved.” The last durable recipe survives restart. Unapplied crop drafts need not survive restart.
- Reopening a catalog restores the image and committed crop/history. Missing sources retain edit state and show a clear unavailable-original message. Detect changed source content before export; do not silently apply a cached recipe to replacement pixels.

## Geometry contract

Choose one canonical, resolution-independent transform, evaluated from the original for every render:

1. Apply embedded EXIF orientation exactly once, including mirrored orientations, producing the **upright source**.
2. Apply the user's horizontal mirror in upright-source coordinates.
3. Apply the user's clockwise quarter-turn count (0–3).
4. Apply clockwise straightening around the image center; proposed range is −45° to +45°.
5. Crop an axis-aligned rectangle in the bounding box of the transformed image.

Store normalized edge coordinates `x`, `y`, `width`, `height` relative to that final bounding box, using top-left origin, x rightward and y downward. Coordinates must be finite; width/height must be positive; the crop's four corners must lie inside the transformed source polygon. UI values and API values refer to exactly this space. The crop aspect ratio is computed in image units, accounting for bounding-box width and height; normalized width/height alone is not the pixel aspect ratio.

Proposed choices: Free, Original, 1:1, 3:2, 4:3, 16:9, plus portrait/landscape swap for fixed ratios. Original means the upright source ratio with quarter-turn orientation accounted for. Fixed ratio values describe exported width divided by height. A numeric custom ratio is future unless the owner wants it in M1.

**Agreed crop controls:** allow free positioning and resizing, including draggable sides and corners. With no aspect ratio locked, a side can move independently and corner dragging can change width and height. Dragging inside repositions the crop. Holding **Option** applies one common scale factor to both dimensions about the fixed center, preserving the crop's current aspect ratio even in Free mode. All four sides move; this is not equal pixel padding on every edge or just mirroring the opposite edge. If a ratio is locked, keep that ratio. Clamp the common factor at the first source boundary rather than shifting the center or stretching one axis. Use Alt as the proposed equivalent on Windows/Linux, subject to platform input checks.

These controls edit a draft with a thirds overlay. Apply/Enter makes one commit; Cancel/Escape discards the draft. Numeric angle/aspect controls are keyboard accessible. No image resampling or history commit occurs for each pointer event. Keep the crop inside valid source coverage so exported JPEGs contain no transparent/black triangles.

**Agreed straightening behavior:** preserve the user's chosen composition as closely as possible and trim only as much as needed to prevent empty corners. Do not reset to a largest centered crop. Retain the composition's center where feasible, maintain a locked aspect ratio, and minimize any necessary movement/reduction. Define the precise fitting objective and tie-breaking in TASK-004 with golden examples. Evaluate every pointer update against the gesture's starting crop/angle snapshot, so dragging the angle away and back does not cumulatively shrink the crop. Do not silently treat this answer as a decision about flip axes or crop behavior under quarter-turns; settle those remaining details in the walkthrough.

Reset Geometry restores the upright source with no user mirror, quarter-turn, straightening, or crop. It is undoable. Undo/redo covers complete committed actions, including automatic crop adjustment. The proposed representation stores a mirror in upright-source axes; the user-facing flip action and its effect on the existing crop remain under interview. Map that action into the final canonical representation once agreed, rather than letting storage order dictate visible behavior.

Define one output-size/rounding convention in TASK-004. Proposed convention: derive crop extent at source pixel scale, floor positive dimensions to whole pixels, sample output pixel centers through the inverse transform, and reject extents below one pixel. For locked ratios, use a documented integer-size fit with at most one-pixel ratio-rounding error. Never repeatedly resample already edited pixels. Cardinal transforms have exact geometry; interpolated angles use the agreed linear-light sampling filter. Preview and export share transform parameters and tests.

## Export and color

Export evaluates the committed recipe and writes a new JPEG at the cropped source resolution, with a simple default quality (proposed 90). No resize presets, watermarking or batch export in M1. The export is an explicit operation; changing the catalog recipe does not continually rewrite JPEGs.

Convert to sRGB and embed an appropriate profile. Normalize/omit orientation metadata so another viewer does not rotate again. Never copy stale dimensions or embedded thumbnails.

**Agreed metadata policy:** strip optional source metadata by default. Provide a **Keep metadata** export setting, initially off, and the equivalent explicit command/API option. With it off, omit source EXIF/IPTC/XMP descriptive data, including capture/camera details, creator/copyright and GPS. Retain or generate the output's required color/geometry information; stripping metadata must not break color or orientation.

With Keep metadata on, preserve supported source descriptive fields (including GPS when present) that remain valid for the edited output, while still updating/removing structural fields and obsolete thumbnails. Define the exact supported fields/containers in TASK-004; do not promise opaque manufacturer/private metadata round trips. The M1 dependency probe must verify a metadata reader/writer path as part of dependency selection, since JPEG pixel decoding alone does not establish metadata support. The export job captures its effective metadata option alongside the recipe revision, and UI/CLI/MCP must produce the same policy. Changing this export option does not modify originals or the geometry recipe.

Never overwrite a source, including through a symlink, hard link, or alternate path. M1 may refuse all existing destinations for simplicity. Write a temporary file in the destination directory, finish/flush it, and publish without replacing an existing file. Failure or cancellation removes temporary output and leaves prior files untouched. Export captures a particular committed revision so later edits cannot change an in-progress job.

Decode/ICC errors and unsupported color modes are explicit. Untagged RGB/greyscale JPEG assumes sRGB, exposed in asset metadata. The selected SDR display contract must be verified for M1; no unsupported promise of wide-gamut/HDR correctness.

## Programmatic parity

Expose import, asset/recipe inspection, geometry update, reset, undo/redo, preview, export, and job status/cancel. Catalog lifecycle operations are also scriptable. JSON commands are proposed and not yet runnable. The service validates ranges and expected revisions regardless of caller. UI pointer state is not required to produce a valid API crop.

M1 requires live control: local IPC routes CLI/MCP operations to the GUI's catalog owner, and committed changes update the visible photo and shared history. A stale revision fails without mutation. If a human has an unapplied crop draft when an agent commits, preserve the draft and show a conflict rather than silently applying or overwriting it. Query session selection, inspect current recipe/revision, request a bounded preview, and perform an edit without GUI automation. A headless owner can run when the GUI is absent; a second owner is rejected.

## Acceptance journey

1. First run the full journey on the M4 MacBook Pro using a packaged native macOS arm64 build; also verify launch without the developer toolchain. Repeat this journey on Windows/Linux under the separate portability gate before claiming those platforms supported.
2. Import a supported JPEG, including EXIF-rotated fixtures. Confirm correct display and a useful first preview.
3. Make quarter-turn, mirror, free/ratio crop and straightening edits with pointer and keyboard controls. Test each side/corner and Option-resize at several zoom percentages, Fit and 100%; verify center/ratio constraints, correct high-DPI mapping, and distinct pan/crop gestures. Use off-center crops and angle sweeps away from/back to the starting angle to check composition preservation without cumulative trimming. Verify real source detail at 100%, no uncovered corners and no UI stalls caused by decode/export.
4. Cancel a draft, apply a draft, undo, redo, and reset. Confirm one meaningful action per history step.
5. Close/reopen the catalog. Confirm the same committed recipe, crop and usable history. Move/rename the fixture, Locate its verified original (also test a matching external-volume copy), and confirm the same asset ID and edits survive. Reject mismatched or ambiguous candidates as specified in the source recovery tests.
6. Export with default metadata stripping and with Keep metadata enabled. Inspect actual metadata tags/containers as well as viewing the result independently; check crop, dimensions, orientation, profile, optional metadata presence/absence and absence of stale thumbnails. Confirm UI/CLI/MCP apply the same option.
7. Repeat equivalent edits through JSON commands and MCP. With the GUI open, have an MCP client inspect the selected image, edit it, request a preview and observe the GUI update; undo the agent edit in the GUI. Exercise simultaneous draft/revision conflicts and reconnects without silent lost updates. Compare canonical recipes and decoded output with declared tolerances; JPEG byte equality is not the color test.
8. Hash source bytes before and after the entire journey; they must match. Attempt export to source aliases and existing destinations; existing files remain unchanged.
9. Exercise malformed JPEG, excessive dimensions, missing/changed original, catalog lock, interrupted commit/export, cancellation and unwritable destinations. Verify clear errors and durable recovery.
10. Record measured results against [performance targets](performance.md), with hardware, build and cache state. Label Linux VM checks separately from native runs and document unverified platform/color paths. Completing the first Mac demo does not complete the Windows/Linux portability gate.
