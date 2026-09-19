# Decision interview

**Current owner override (2026-09-19):** manual Windows/Linux desktop checks and manual license reviews are deferred. S0 currently requires native M4 evidence and automated portable checks; deferred checks are not passing results. See [hardening scope](engineering/s0-hardening.md).

Status: **interview in progress; first-build scope revised**. The latest 2026-09-19 instruction makes S0 a cross-platform skeleton that loads an image with no tools. The earlier geometry/persistence/export/live-agent demo remains the subsequent M1 editor, including agreed crop, metadata and zoom behavior. Product choices and implementation now have separate task files. GPL-3.0-or-later is now selected by the owner. M1 workspace, geometry, history, export and conflict defaults were subsequently adopted; later product choices remain open.

## Already established by the owner

- Cross-platform desktop: Windows, macOS and Linux.
- Open source, free to modify, highly performant, simple and extensible.
- Professional/prosumer catalogs, potentially multiple terabytes of originals.
- Every application operation must be available to code/agents, including all bundled and external photo edits; see D19–D21 for full API and host/module requirements.
- First test: one JPEG in a catalog with simple geometry edits; later PNG and Nikon/Fujifilm RAW.
- Lightroom is a UX baseline; Library/Develop are relevant, the other named modules and Publish Services are not.
- Markdown documentation and design/specification before work; JSON task plans through Create Tasks.
- Everything is v0; public API stability and release-version planning are premature.
- The first target is the owner's M4 MacBook Pro. Linux VM testing is desirable, with Omarchy suggested as a candidate. Windows and Linux remain product targets.
- First RAW cameras: Nikon Z6 (original model) and Fujifilm X100VI. Prefer existing libraries; a custom RAW library is an option if evidence establishes inadequate performance or support.
- Ideally all project software and extensions should be open source; a proprietary-fork/plugin ecosystem is not the desired direction.
- First end-to-end build is now S0: a cross-platform skeleton that opens/displays an image with no editing tools. The earlier crop/straighten/rotate/flip, undo/reopen/export scope remains M1.
- Agents must be able to edit while the GUI is open in M1. S0 first establishes development logs, state and screenshots for agent verification; production IPC/MCP follow in M1.
- Reference existing originals initially. Recovery from externally moved/reorganized photos is a major product concern, not incidental error handling.
- Manual **Locate missing original** is in M1; folder relinking and automatic detection follow later. The owner usually edits local files then syncs back to an external drive; research broader photographer workflows before choosing managed storage/sync behavior.
- Crop provides free control and draggable sides. Holding Option scales the whole crop proportionally around its fixed center, preserving its current aspect ratio.
- Straightening preserves the chosen composition as closely as possible and trims only enough to avoid empty corners; it does not reset to a centered crop.
- Export strips optional source metadata by default, with a setting to keep it.
- M1 includes basic zoom/pan, percentage settings and Fit; zoom is no longer an optional first-demo feature.

## First interview: constraints that change architecture

| ID | Owner answer / status | Remaining detail | Consequence |
| --- | --- | --- | --- |
| D1 | **Decided:** M4 MacBook Pro first; consider Linux VMs, possibly Omarchy | Exact RAM/chip variant, minimum OS/older hardware, and later Windows/Linux test machines are not yet specified | Develop on macOS arm64 first; the revised S0 gate requires the accepted three-platform launch/load matrix, with M1 editor portability rechecked later |
| D2 | **Decided:** Nikon Z6 and Fujifilm X100VI; prefer library reuse, allow custom work if justified | Actual RAW compression, bit depth and sample/firmware variants remain unverified | Both models appear on LibRaw's support list; test real modes and profile decoding before replacing a library |
| D3 | **Decided:** GPL-3.0-or-later; owner accepted the recommendation during the scaffold interview | Apply the license text/manifest declaration in TASK-005 and audit the configured build in TASK-041 | Use compatible open-source project/runtime dependencies and bundled extensions; no prototype-only license deferral |

These answers replace the earlier hypothetical 16 GB baseline and unresolved camera/license-intent questions. They do not establish a minimum RAM requirement or prove library/VM performance. Sources and licensing boundaries are in [technical research](research/technical-options.md).

### Decision history — 2026-09-19

Recorded the first owner answers and propagated them to the plan, platform acceptance, performance workloads, RAW strategy, feature status and task context. M1 becomes the first working Mac demo; Windows/Linux remain required for a separate portability claim. No VM has been installed and no decoder benchmark has run. The license recommendation has not yet been applied as a repository license.

Subsequent answers confirmed the full geometry/undo/reopen/export demo, by-reference import, live agent control and manual Locate. Added explicit IPC, MCP and source-recovery tasks and a recovery specification. Record local editing followed by external-drive sync as the owner's workflow, while keeping broader storage workflow research as future work.

The next round confirmed free crop handles, metadata stripped by default with a keep option, and zoom/pan with percentage settings and Fit. Follow-up answers resolved Option resizing as proportional scaling around the center and straightening as preserving composition with minimum necessary trimming. Updated specifications and task acceptance criteria accordingly.

## Second interview: challenge the workflow

Ask these after the initial constraints, with the researched tradeoffs available. Avoid turning the whole table into a single questionnaire.

| ID | Decision and concrete tradeoff | Proposed starting point |
| --- | --- | --- |
| D4 | **Decided, 2026-09-19:** reference existing files; manual Locate in M1. Owner edits locally then syncs to an external drive | Stable asset identity and verified relinking; folder recovery and broader storage/sync workflow research later. See [recovery design](specs/source-recovery.md) |
| D5 | **Scope retained, sequence superseded by D17:** crop/straighten/rotate/flip, undo, reopen and export are M1 | S0 loads/displays first. Crop resizing, straightening and metadata remain agreed; visible flip and quarter-turn crop behavior remain open |
| D6 | **Decided, 2026-09-19:** live agent control while the GUI is open is part of M1, after S0 | Add explicit local IPC and MCP tasks, shared revision/history semantics, event updates and UI-draft conflict handling. The earlier GUI-closed CLI limitation is superseded |
| D7 | Should RAW arrive immediately after geometry, or should we first have a useful small library and JPEG tonal tools? How important is matching the camera JPEG/film simulation? | Research exact cameras early; decide M3/M4 order from daily-use value |
| D8 | Is a fresh, small core worth slower progress toward mature RAW quality, or should we consider adapting darktable/RawTherapee components under compatible licensing? | New small core with existing libraries; reconsider larger-engine reuse if RAW quality dominates |
| D9 | Is there a language/contributor preference? Would you accept UI framework maintenance to keep everything Rust, or prefer Qt if desktop behavior is stronger? | Rust + Iced trial, egui comparison, Qt fallback; select from evidence |

## Third interview: behavior and visual decisions

| ID | Owner answer / status | Remaining detail |
| --- | --- | --- |
| D10 | **Decided:** free crop control with draggable sides; Option scales proportionally about the fixed center | Preserve the starting crop's aspect ratio; uniformly clamp expansion at source bounds. Alt is the proposed Windows/Linux equivalent |
| D11 | **Decided:** straightening preserves composition and trims only as needed to avoid empty corners | The M1 geometry experiment must define a deterministic fitting algorithm and golden examples; no reset to a centered crop |
| D12 | **Decided:** strip optional metadata by default; allow a keep-data setting | Output color/geometry information remains correct in both modes. Implement a documented supported-field policy for keeping metadata rather than copying stale orientation/dimensions/thumbnails |
| D13 | **Decided:** basic zoom/pan, percentage settings and Fit in M1 | Include numeric percentage control and 100% inspection. Input gestures, limits and high-DPI pixel mapping are implementation details to validate for M1 |
| D14 | **Decided:** one workspace with collapsible editing panels; no full grid in M1 | Recommend one workspace with collapsible library and editing panels. This does not add a full grid to M1 |
| D15 | **Decided:** mirror the visible composition left-to-right and carry its crop | Recommend mirroring what the user sees. TASK-004 finalizes the mathematical command mapping; canonical storage order must not dictate visible behavior |
| D16 | **Answered:** workflow frustrations recorded below, including speed, bloat, library organization and export | Use concrete examples to prioritize workflow improvements and later milestone order |

- How should quarter-turn rotation transform an existing crop, and which Lightroom shortcuts should transfer?
- Which aspect-ratio presets and numeric crop controls are essential? Free dragging must remain available when no ratio is locked.
- Must edits travel beside photos as sidecars from the start, or can the local catalog be authoritative until portability is scoped? How do you back up catalogs and originals today?
- What is the first extension you would actually install or write: a preset, custom panel, workflow automation, RAW decoder, or pixel-processing tool? Design that path first.

## Latest scope change: bootstrap before editing

| ID | Owner answer / status | Consequence |
| --- | --- | --- |
| D17 | **Decided:** the first end-to-end build is a cross-platform skeleton that loads an image; no tools needed | Add S0 before M1. A transient Open image/Fit workflow replaces the full editor as the first build. Preserve earlier M1 decisions; no catalog, editing tools or production MCP prerequisite for S0 |
| D18 | **Decided:** separate product decision tasks from implementation and detail setup, tooling, linting, builds and agent testing | Create two active JSON plans with explicit decision gates; add real-render screenshot/log/state evidence, native platform checks and a contributor playbook |

The latest sequencing supersedes the earlier Mac-only first-demo plan, not the product's editing requirements. The earlier history above describes the previous plan. No code, VM, benchmark or build was produced by this restructuring. The word “lighting” is interpreted as linting in the tooling request; this is an explicit working interpretation, not a new photo feature.

See [product decision tasks](../tasks/product-decisions.json), [implementation tasks](../tasks/implementation.json), [S0 scope](specs/bootstrap.md) and [dependency rules](task-planning.md). S0 needs only the platform, license-disposition and minimal shell decisions; the full editor interview continues separately.

## Architecture reaffirmed: programmable operations and modules

Owner instruction, **2026-09-19**: every operation must be exposed through an API; the application should be a small shell/module host whose shared core applies and undoes non-destructive changes; prefer user-selectable modules where worthwhile, but avoid needless modularization for an unproven performance gain. External modules must still be loadable and use core APIs.

| ID | Owner answer / status | Consequence |
| --- | --- | --- |
| D19 | **Decided:** all operations are programmable, especially photo edits including exposure, white balance, cloning and masks | Every feature, bundled or external, supplies structured operations/state through the common service. No tool depends on GUI gestures. Future tool examples do not expand M1 scope |
| D20 | **Decided:** a small shell/module host and basic shared core underpin the software | Core owns recipe transactions, shared history/undo and services; modules implement their own parameters, validation and algorithms using those APIs. Replace the ambiguous claim that every effect belongs in the core |
| D21 | **Decided:** prefer useful user-selectable modules, conditional on meaningful benefit for core splitting; support external loading regardless | Separate logical modules, runtime enablement, lazy resources and binary packaging. Preserve edits when modules are missing/disabled. Measure before claiming speed or fragmenting basic features into plugin binaries |

**Engineering recommendation, not a measured result:** keep lightweight built-ins linked, make registration cheap, and initialize expensive resources on demand. Optional presets, decoder/resource groups and workflow integrations are useful candidates. A per-camera split may save little when cameras share a decoder. Choose granularity from evidence rather than one binary per tool. An Instagram export module illustrates optionality; it is not a newly accepted Publish Services feature.

**Still open:** the first external use case, exact optional boundaries and performance tradeoffs, runtime/package format, trust/isolation and UI contribution mechanism. External loading itself is no longer an open question. Existing sequence remains S0 viewer, M1 host contracts and built-in tools, then the scoped external proof currently placed in M5. No marketplace, stable public ABI or hot unloading requirement is added.

The [module/API design](design/modules-and-api.md) records responsibilities, API examples, lifecycle protections and acceptance. Updated architecture, plan, feature status, editor/performance specs and contributor/user guidance. Product TASK-001/030/033/034 preserve these answers while retaining their unresolved work. Implementation TASK-007/008/009/011/013/014/016/021/064 enforce them in M1; new TASK-067 tracks activation measurements and the later external-loader implementation plan. Existing task IDs, completed history and S0 scope are preserved; this is documentation/planning work only.

## How decisions become authoritative

Record each answer with its rationale, affected documents/tasks, and status (open, decided, or superseded). Necessary behavior questions must be resolved before their implementation task starts. This is a documentation dependency, not a new approval ceremony for routine work. Prototype conclusions and owner preferences can revise the recommendation without preserving premature compatibility.

## Scaffold interview: license decision

The owner answered “Use recommendation” to the one-at-a-time choice between GPL-3.0-or-later and deferred license selection. **GPL-3.0-or-later is selected**, completing product TASK-024. This answer does not accept the previously bundled platform/layout proposal or authorize the blocked native UI action.

Keep project/runtime dependencies and extensions open source and compatible with the selected license. Inspect bundled fonts/assets, native libraries, optional features and extension integrations as part of TASK-041; preserve their required notices and exclude incompatible components. Native macOS/Windows SDKs and development toolchains are documented separately from redistributed project/runtime code. No proprietary hosted service is required for development or testing. TASK-005 applies the repository license text and manifest declaration; selection is not evidence that the configured dependency audit has passed.

## Scaffold interview: initial platform targets

The owner answered “Yes” to **macOS on Apple Silicon, Windows x64, and Linux x64, with unsigned development packages**. These targets and distribution scope are accepted for S0. This does not establish minimum OS/runtime versions, Linux X11/Wayland coverage or successful desktop verification. TASK-023 remains in progress until those matrix details and test routes are recorded; missing Windows/Linux sessions must remain explicit. The M4 macOS test route already has experimental Metal-render evidence.

## Scaffold interview: minimal window behavior

The owner answered “Yes” to a dark background, one **Open image** button, automatic Fit display, brief loading/error messages, and retaining the previous photo when a replacement open fails. These S0 shell behaviors are accepted. This does not approve the experimental exact-profile allowlist as the final supported JPEG contract. TASK-025 remains in progress for that input-policy decision. Proposed initial window size is 960×640 logical pixels with standard platform Open keyboard behavior; these routine implementation choices remain subject to native verification.

## Scaffold interview: supported JPEG input

The owner answered “Yes” to standard sRGB and greyscale JPEGs, automatic orientation correction, and a clear error for unsupported color profiles, with broader color support later. Combined with the accepted shell behavior, this completes product TASK-025. S0 targets 8-bit RGB/greyscale JPEG, EXIF orientations 1–8, untagged assumed sRGB and supported standard tagged sRGB. Unsupported or malformed profiles must be rejected explicitly. Opening remains transient and creates no catalog.

The probe's exact generated-profile allowlist is an experimental limitation, not sufficient implementation of general standard-sRGB support. TASK-005 must select a defensible profile recognition/display path, and subsequent correctness checks must cover independently sourced/generated standard-sRGB variants before claiming the accepted subset implemented.

## Scaffold interview: provisional platform baselines

The owner answered “Yes fine for now” to allowing engineering to choose and document initial minimum OS versions and Linux desktop configuration from the selected framework's requirements. This delegates the initial technical selection; it does not claim any version has been tested. Record concrete baselines, artifact formats and available/missing test routes before completing TASK-023 and TASK-035. Revisit broader compatibility after the scaffold. Native Windows/Linux desktop evidence remains required for S0 completion; no VM installation or purchase is authorized by this answer.

## Scaffold interview: local execution approval and matrix record

The owner explicitly approved opening the local Iced trial and stated that locally built/running software is approved. The previous approval-review block is resolved by that answer. A subsequent computer-use attempt reported the Mac locked; interaction checks await an unlocked desktop. No further permission question is needed for the same local build/test scope.

Under the delegated baseline choice, engineering recorded macOS 14+ arm64, Windows 11 24H2+ x64, and Ubuntu 24.04 x64 with GNOME Wayland and X11 validation, plus development archive formats and explicit missing test routes in the [platform matrix](engineering/platforms.md). This completes product TASK-023 as a decision record. It does not complete platform verification. Product TASK-023/024/025 are now all complete, allowing implementation TASK-035 to close.

## Scaffold hardening scope override — 2026-09-19

The owner instructed: “Update tasks to not require manual windows/linux checking for now. Don't worry about licenses or reviews of them for now. Complete all the tasks you listed.” TASK-058/059 and review task TASK-041 are deferred with their IDs and unfinished criteria retained. They are removed from the current S0 dependency gate. Automated portable builds and existing notices remain. Existing automated dependency checks and expiring maintenance exceptions remain in place; no new license review is performed during this slice. This is no change to GPL selection and no assertion of license compatibility or native Windows/Linux verification.

Implementation proceeds on local hardening, packaged Mac verification and measurements. M1 recommendations have been presented for TASK-026–030; absent an answer, they remain proposals. Personal Lightroom frustrations cannot be inferred from an instruction to execute engineering tasks.

## Owner workflow priorities — 2026-09-19

The owner identified these Lightroom frustrations beyond moved-source recovery: speed; unused-tool bloat; poor filtering; confusing colour/rating/flag tagging; unintuitive collections; inability to retain a full catalog while lazily loading a subsection such as one shoot for editing; awkward multi-selection/stacking; no native identification of bracketed exposures or panoramas; and confusing export controls without modern defaults.

These are accepted **problem statements and prioritization input**, not approval of every proposed solution or a commitment to implement these features in S0/M1.

| Priority input | Immediate consequence | Later design work |
| --- | --- | --- |
| Speed and unused-tool bloat | Measure startup/loading/idle; keep S0 minimal and M1 limited to agreed tools | Measure module activation and lazy resource use; avoid speculative binary splitting |
| Filtering, tagging and collections | Do not copy Lightroom's organization model by default | M2 interview must compare concrete retrieval tasks and propose a small consistent model |
| Full catalog with lazy shoot/subsection editing | Preserve catalog-scale independence from decoded pixels | M2 queries and virtualized views should scope a shoot without loading the entire catalog; determine shoot/group identity and navigation |
| Multi-selection and stacking | Keep out of one-image M1 | Specify selection ranges, stack identity, representative image and batch-action behavior for M2 |
| Bracket/panorama identification | Record as a workflow candidate | Research detection/grouping separately from HDR or panorama merging, which remain unselected |
| Confusing export/defaults | Present one clear M1 JPEG export path with the agreed metadata default | Confirm quality/name/collision defaults through TASK-029; later platform destinations are separate scope |

TASK-026's personal-frustration input is now answered. Workspace adoption, geometry details, history/recovery, export defaults and conflict presentation still await the separate recommendations question. Do not interpret this answer as approval of those defaults.

## M1 recommendations adopted — 2026-09-19

The owner answered **“Adopt these recommendations and choose routine details”** to the explicit TASK-026–030 proposal: one workspace; flip/rotate carry the visible crop; Apply/Cancel with persistent committed undo; JPEG quality 90 with no overwrites; preserve human drafts and show conflict when an agent commits. The separately answered Lightroom-frustration input completes the personal-workflow criterion. Product TASK-026–030 are now complete.

The [accepted M1 decisions](design/m1-decisions.md) record routine shortcuts, focus behavior, crop presets/custom ratio, Space-drag pan, ±45° fine straightening, nonpersistent drafts, verified manual Locate/duplicate handling, export naming/metadata principle and shared attributed history/reconnect behavior. Exact geometry algorithms, color/metadata feasibility and numerical tolerances remain engineering work in TASK-004. S0 and TASK-064 remain gates; this acceptance implements no editing tools. Earlier open/proposed interview entries are historical and superseded by this record.

## Development tooling — Rust

Owner decision: implement all project development tooling in Rust, using the existing xtask entry point. Use the same Rust commands for local development and CI. An independent Go client for external API testing is a possible later addition, not a current prerequisite. TASK-069 implements this decision.

## M1 history as a core editor feature — 2026-09-19

The owner clarified that undo/redo alone is insufficient: every committed edit, crop, flip and other image action must enter a shared historical log, with preview and restore at any point. This is an accepted M1 requirement for the host and every tool; later tools inherit it when introduced. Original and complete intermediate recipes must remain inspectable after reopening and further editing. Transient pointer movements and viewport changes remain outside image-edit history.

The [coverage review and history contract](specs/edit-history.md) identify missing explicit tasks for history browsing, arbitrary-entry preview/restore, retention outside the shortcut redo path and API/UI verification. Existing core tasks are strengthened and TASK-071 adds the history interface. TASK-028/030 keep their accepted decision history. The subsequent owner answer on 2026-09-20 explicitly accepts appending an explicit Restore action and retaining all later actions, completing product TASK-070. Undo/redo navigates saved entries without deleting them; a new edit clears shortcut redo availability while retaining historical access. TASK-064 must verify TASK-070 before completing. This updates plans only; no editor functionality is implemented.
