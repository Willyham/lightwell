# Feature status

As of 2026-09-19: the maintained Rust/Iced viewer builds and renders on native M4 Metal. Open/Fit, bounded decode and synthetic smoke evidence are implemented locally; S0 acceptance is still in progress. See [working scaffold commands](engineering/scaffold-commands.md) for evidence and limitations.

Status vocabulary: **planned now** = S0 bootstrap scope; **planned next** = retained M1 task scope after the skeleton; **planned later** = roadmap intent without an implementation-ready task list; **candidate** = needs a product decision; **excluded** = deliberately outside the present product direction; **implemented** requires verification evidence. Experimental rows are not production application claims.

| Capability | Status | Milestone / completion reference |
| --- | --- | --- |
| Repository conventions and plan/link checks | Implemented | TASK-037; Python checker, contributor instructions, ignore/editor defaults |
| Reproducible synthetic JPEG corpus | Implemented | TASK-002; 16 fixtures and generated 24/60 MP inputs |
| JPEG and Iced/egui technical probes | Experimental | TASK-036 complete; TASK-003 complete; Iced selected |
| Native macOS arm64 build | Implemented locally; acceptance ongoing | S0 package/load verification on M4; M1 editor verification follows |
| Windows/Linux packages and launch/load checks | Planned now | S0 requires the accepted matrix; TASK-019 later rechecks M1 editor behavior |
| Linux VM checks, possibly Try Omarchy | Candidate | Functional testing on Apple Silicon; no installed/tested VM and no native GPU-performance claim |
| License selection and dependency policy | License applied; audit in progress | GPL-3.0-or-later; two unmaintained dependency findings and asset review remain |
| Open image and automatic Fit display | Implemented on macOS; wider verification pending | S0; no catalog, editing tools or export; [skeleton spec](specs/bootstrap.md) |
| Pinned tooling, setup/Doctor and command runner | Implemented locally | S0 developer workflow |
| Formatting, linting, dependency and documentation checks | Implemented; audit findings open | S0 local/CI parity |
| Structured logs, state/readiness and actual-render screenshots | Initial evidence implemented; hardening pending | S0 agent verification; [tooling contract](engineering/development.md) |
| Process-level smoke tests and retained CI evidence | Native macOS smoke verified; CI not executed | S0; unsupported GUI checks remain explicit |
| One-image catalog and JPEG import | Planned next | M1; [spec](specs/single-image.md) |
| Non-destructive geometry recipe and persistence | Planned next | M1 |
| 90° rotation and horizontal flip | Planned next | M1 |
| Crop, aspect ratios and fine straightening | Planned next | M1; free side/corner handles, proportional centered Option-resize and composition-preserving straightening agreed |
| Zoom/pan, percentage controls and Fit | Planned next | M1 confirmed; includes 100% inspection and high-DPI coordinate checks |
| Undo/redo and reset | Planned next | Undo confirmed in M1 demo; redo/reset specified alongside it |
| JPEG export | Planned next | M1 confirmed; strip optional metadata by default with Keep metadata setting |
| Input/display/output color correctness for supported JPEGs | Planned next | M1 color proof/implementation; S0 declares a smaller sRGB viewing subset |
| JSON command/query API and headless operation | Planned next | M1; clients route to one catalog owner |
| Bounded image loading and baseline measurements | Planned now | S0 bounds and M4 baseline; broader M1 measurements follow |
| Decoder/tool module boundaries | Planned next | M1 built-ins; no dynamic plugin system |
| Multi-image import and virtualized Library/filmstrip | Planned later | M2 |
| Metadata filters and saved smart filters | Planned later | M2; filter dimensions to specify |
| Ratings/flags/labels/collections | Candidate | Choose the smallest useful organization set for M2 |
| Local IPC, live GUI/agent coordination and MCP | Planned next | M1 confirmed by owner; shared revisions/history and conflict handling |
| Thumbnails, preview pyramid, bounded disk cache | Planned later | M2; minimal preview in M1 |
| Exposure and white balance temperature/tint | Planned later | M3; RAW versus rendered-image semantics differ |
| Contrast, highlights, shadows, whites, blacks | Planned later | M3; algorithms and order to specify |
| Vibrance and saturation | Planned later | M3 |
| PNG import/export | Planned later | M3; alpha, bit depths and profiles need a spec |
| Nikon Z6 NEF and Fujifilm X100VI RAF development | Planned later | M4; models decided, recording modes and quality criteria pending; existing-library trial first |
| Custom RAW decoder/library | Candidate | Only after benchmarked support/performance gaps and comparison with existing libraries |
| Texture, clarity and dehaze | Planned later | M5; scale/neighborhood and quality work |
| Tool visibility/order and resettable default workspace | Planned later | M5, potentially earlier for built-ins |
| Presets and external workflow extensions | Planned later | M5; first use case to choose |
| WASM/native image-processing extensions | Candidate | Separate extension design after a real use case |
| Manual Locate for moved/reorganized originals | Planned next | M1 confirmed; [recovery spec](specs/source-recovery.md), original ID/edits preserved |
| Folder relinking, move detection, multiple source locations and NAS workflows | Planned later | Research varied photographer workflows; active catalog stays local initially |
| Sidecars, catalog portability and managed copy import | Candidate | Owner workflow and backup decisions |
| Sharpening, denoise, lens correction and camera looks | Candidate | RAW-quality interview; not silently included |
| Semantic search/AI culling or local model inference | Candidate | Agent access does not require bundled AI features |
| Lightroom catalog/XMP edit migration | Candidate | No claim of Adobe rendering or parameter compatibility |
| HDR display/export and soft proofing | Candidate | Separate color/output scope |
| Masks, healing, panorama, HDR merge, tethering | Candidate | No current implementation commitment |
| Map, Book, Slideshow, Print, Web and Publish Services | Excluded | Owner explicitly deprioritized these modules |
| Mandatory accounts/cloud sync/model subscriptions | Excluded | Outside the proposed local-first core |
| Plugin marketplace and collaboration service | Excluded | No present product need |

Track [product decisions](../tasks/product-decisions.json) and [implementation](../tasks/implementation.json) separately; [task sequencing](task-planning.md) defines gates and history. Completing research or a prototype does not move a production feature to implemented. Later milestones get their own Markdown specifications and task plans when selected.
