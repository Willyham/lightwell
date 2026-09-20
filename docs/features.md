# Feature status

As of 2026-09-20, S0 is owner-accepted and the Rust/Iced viewer opens supported JPEGs at Fit on native M4 Metal. The editor milestones below are **planned and on hold**.

| Capability | Status | Scope |
| --- | --- | --- |
| Repository/toolchain, Rust checks, fixtures and diagnostics | Implemented | S0; [working commands](engineering/scaffold-commands.md) |
| JPEG Open, automatic orientation, Fit and failed-replacement retention | Implemented on native M4 | S0 supported sRGB/greyscale subset |
| Native macOS package, rendered smoke and baseline measurements | Verified locally; S0 accepted | [hardening evidence](engineering/s0-hardening-results.md) |
| Windows/Linux automated builds/packages | Earlier hosted scaffold verified; refresh remains open | Native desktop checks deferred |
| License and dependency policy | GPL applied; policy checks implemented | Manual license/native/asset review deferred; timed maintenance exceptions remain |
| Stable referenced assets and local catalog | Planned M1 | SQLite persistence, fingerprints, explicit missing/changed-source state |
| Ordered non-destructive edit layers | Planned M1 | Stable layer IDs and immutable complete recipe snapshots |
| Test pixel-change tool | Planned M1 | Integer x/y and sRGB color; exact lossless-buffer proof |
| Persistent action history, undo/redo and append-only Restore | Planned M1 | Every real committed action; retained historical branches |
| History list/inspect/select/preview UI and API | Planned M1 | Read-only preview, current/selected markers, Return to current |
| Save/reopen layers, history and navigation state | Planned M1 | Atomic catalog writes and failure recovery |
| Fit, numeric zoom, 100% source detail and pan | Planned M1 | Needed to inspect the test pixel; independent session state |
| Live external JSON API and one-owner IPC | Planned M1 | Same service/history while GUI is open |
| Rotate left/right, Mirror horizontal, Flip vertical | Planned M2 | Exact discrete mappings and shared history |
| Declarative tool-module interface | Planned M3 | Action/API/control descriptions, validation and processing |
| Pixel and transform tool modules | Planned M3 | Preserve previous effect identities, snapshots and pixels |
| Lightroom-style crop/straighten module | Planned M4 | Free handles, ratios, angle/guide, Apply/Cancel and reset |
| Proportional centered Option crop scaling | Planned M4 | Fixed center/common scale and source-boundary clamp |
| Composition-preserving straightening and draft conflicts | Planned M4 | No cumulative trim; explicit resolution after agent commits |
| Safe JPEG export, color/metadata verification | Planned editor follow-up | Quality 90, no overwrite, strip optional metadata by default, Keep metadata |
| Manual Locate | Retained editor follow-up | Verified content, stable identity/edits; [recovery contract](specs/source-recovery.md) |
| Standards-compliant MCP adapter | Retained editor follow-up | Common registry/owner, live UI/headless parity |
| Complete editor packaging and performance/portability verification | Retained editor follow-up | Native M4 handoff; Windows/Linux native scope remains deferred |
| External module loading | Required later | Separate selected-use-case proof with measured activation costs |
| Multi-image library, filters, tagging, collections and shoot subsets | Planned later | Scope through owner workflow decisions |
| Exposure, white balance, tonal controls and PNG | Planned later | Tool-specific numerical and color contracts |
| Nikon Z6 / Fujifilm X100VI RAW | Planned later | Actual recording-mode fixtures and established-decoder benchmarks |
| Texture, clarity, dehaze, masks and clone/heal | Future scoped work | Complete APIs required whenever introduced |
| General bitmap layers, blend modes and layer reordering | Not selected | “Layers” currently means ordered recipe operations |
| Sidecars, sync, managed-copy import and folder relinking | Later decisions | Local catalog and by-reference import first |
| Plugin marketplace, mandatory cloud/accounts, Map/Book/Print/Web modules | Excluded from current scope | No prerequisite for the editor |

The [four-milestone design](design/history-first-roadmap.md) and [task index](../tasks/README.md) are authoritative for ordering. [Lightroom research](research/lightroom/README.md) and [darktable research](research/darktable/README.md) inform decisions; they do not establish implemented features.
