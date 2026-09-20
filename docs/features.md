# Feature status

| Capability | Status | Notes |
| --- | --- | --- |
| Repository checks, fixtures, smoke evidence, packaging | Implemented | Smoke and hardening run against the editor; [development](engineering/development.md) |
| JPEG open, automatic EXIF orientation, Fit, failed-replacement retention | Implemented (S0) | 8-bit sRGB and greyscale subset; broad ICC conversion and display calibration are not established |
| Windows/Linux automated builds and packages | Hosted baseline verified; refresh open | Native desktop checks deferred |
| License and dependency policy | Enforced | Manual license, native and asset review deferred; two expiring advisory exceptions ([dependencies](engineering/dependencies.md)) |
| Referenced assets in a SQLite catalog | Implemented (M1) | Stable IDs, fingerprints, explicit missing or changed-source errors |
| Ordered non-destructive edit layers | Implemented (M1) | Stable layer IDs, immutable complete recipe snapshots |
| Test pixel-change tool | Implemented (M1) | Integer x/y and 8-bit sRGB; exact lossless-buffer proof |
| Persistent history, undo/redo, append-only Restore | Implemented (M1) | Every committed action; all branches retained |
| History list, inspect, select and preview in UI and API | Implemented (M1) | Read-only preview and Return to current |
| Save and reopen layers, history and navigation | Implemented (M1) | Atomic catalog writes |
| Fit, numeric zoom, 100% source detail and pan | Implemented (M1) | Physical-pixel 100%; per-client session state |
| Live JSON API and single-owner IPC | Implemented (M1) | Same service and history while the GUI is open |
| Rotate left/right, mirror horizontal, flip vertical | Implemented (M2) | Exact integer mappings |
| Named versions and lineage view | Implemented | A version names a retained entry; lineage query and branch markers; catalog format 2 ([design](design/versions-and-lineage.md)) |
| Cached source and compiled one-pass rendering | Implemented | Point queries never rasterize; [performance rules](engineering/performance-rules.md) |
| Declarative tool-module interface | Implemented (M3) | Descriptor-validated registry, generated `edit.<action>` methods and `module.list`; [modules](design/modules-and-api.md) |
| Pixel and transform tools as modules | Implemented (M3) | Effect and action identities unchanged; pre-module golden journey reopens identically |
| Generic module controls and canvas pick in the desktop | Implemented (M3) | Controls rendered from descriptors; pointer pick fills coordinates without committing |
| Lightroom-style crop and straighten module | Planned (M4) | [crop contract](specs/single-image.md) |
| Draft conflicts with live agent commits | Planned (M4) | Explicit Discard or Reapply |
| JPEG export with color and metadata verification | Editor follow-up | Quality 90, no overwrite, Keep metadata option |
| Manual Locate | Editor follow-up | [source recovery](specs/source-recovery.md) |
| MCP adapter | Editor follow-up | Same operation registry; the JSON API is not MCP |
| Complete editor packaging and portability verification | Editor follow-up | Native Windows/Linux remains deferred |
| External module loading | Required later | Selected use case and measured activation costs first |
| Multi-image library, filters, tagging, collections | Later | Owner workflow decisions first |
| Exposure, white balance, tonal controls, PNG | Later | |
| Nikon Z6 and Fujifilm X100VI RAW | Later | Real recording-mode fixtures; benchmark established decoders |
| Texture, clarity, dehaze, masks, clone/heal | Later | Full API required whenever introduced |
| Bitmap layers, blend modes, layer reordering | Not selected | |
| Sidecars, sync, managed-copy import, folder relinking | Later decisions | |
| Marketplace, cloud or accounts, Map/Book/Print/Web modules | Excluded | |

Unverified so far on every milestone: native Windows/Linux desktop behavior, native screen-reader exposure of the custom controls, minimum-OS execution and calibrated display color.
