# Lightwell v0 project plan

**Current owner override (2026-09-19):** manual Windows/Linux desktop checks and manual license reviews are deferred. S0 currently requires native M4 evidence and automated portable checks; deferred checks are not passing results. See [hardening scope](engineering/s0-hardening.md).

Status: **S0 implementation in progress**. Rust/Iced is selected in the [stack decision](design/s0-stack.md). The maintained viewer runs on macOS; [working commands and limitations](engineering/scaffold-commands.md) separate verified behavior from remaining acceptance work. Research checked 2026-09-19.

## Recommendation

Start with a native Rust application, a UI-independent command and image engine, SQLite for catalog metadata and edit recipes, and a custom GPU-rendered desktop interface. Trial **Iced + wgpu** first, with **egui + wgpu** as a small comparison prototype. Keep **Qt Quick** as the mature alternative if the Rust UI candidates fail essential desktop requirements. The S0 trial now selects Rust/Iced; later catalog/editor choices remain proposals. [Research and sources](research/technical-options.md#desktop-interface-options).

Use an existing JPEG decoder initially. Preserve decoder boundaries for PNG and RAW. Trial **LibRaw** for the owner's **Nikon Z6 and Fujifilm X100VI**, while treating high-quality RAW development as separate engineering. Use **Little CMS** for ICC color conversion. Prefer existing libraries; profile and compare alternatives before a targeted fork or new RAW library. Custom decoding remains an option if measured performance or support gaps justify it. [Image dependencies](research/technical-options.md#image-processing-and-raw).

Target the **M4 MacBook Pro on macOS arm64 first** for development, with the proposed wgpu Metal path. The first end-to-end build retains automated Windows/Linux builds and packages; manual desktop checks are deferred by the owner. A Linux ARM64 VM, including the current Try Omarchy candidate, can supply labelled guest functional checks but does not establish native Linux GPU performance. The owner has selected **GPL-3.0-or-later**; the repository license is applied and the configured dependency audit remains in progress.

The owner's latest instruction makes the first deliverable **S0: a cross-platform skeleton that opens and displays a JPEG, with no editing tools**. It includes project setup, tooling, linting, repeatable builds and agent verification through structured logs, state and real-render screenshots. Opening is transient; no catalog is needed. See the [skeleton specification](specs/bootstrap.md) and [development workflow](engineering/development.md).

The previously agreed persistent editing loop becomes the subsequent **M1**: reference a JPEG in a catalog, crop/straighten/rotate/flip it, undo, reopen it and export a new JPEG, with an agent able to edit while the GUI is open. Local IPC/MCP, interactive zoom and manual Locate remain M1 requirements; they do not block S0.

Originals stay where the user keeps them. The owner identified moved/reorganized photos as a major catalog pain point, so separate photo identity from file location immediately. **Manual Locate is agreed for M1**; the [source recovery design](specs/source-recovery.md) covers verified relinking and duplicate ambiguity. The owner's local-edit/external-archive workflow is an initial example, with broader workflow research, folder recovery and automatic detection deferred.

## Confirmed goals

1. Free, modifiable, open-source desktop software on macOS, Windows, and Linux.
2. Fast interaction and practical catalogs spanning hundreds of gigabytes to multiple terabytes of originals.
3. Every application operation available to programs and agents, with inspectable state and results: all edits, catalog/workflow actions, session/settings and module lifecycle, including capabilities added by external modules.
4. Simple defaults, beautiful software, and a small shell/module host with shared non-destructive services. Feature modules use the core APIs; externally loaded modules are a required extension path.
5. JPEG first; a modular path to PNG, Nikon Z6 RAW, and Fujifilm X100VI RAW.
6. Documentation, design specifications, and JSON task plans before implementation. Clear visibility of unfinished features.
7. M4 MacBook Pro first; open-source-only project and extension direction. GPL-3.0-or-later is selected; recording modes and minimum hardware remain to be finalized.

Original byte volume is not the main catalog-sizing variable. We must measure photo count, metadata/filter selectivity, preview storage, image resolution, and storage latency separately. A catalog can reference a multi-terabyte archive without loading those pixels into RAM.

## Product shape

S0 exposes an Open image action and a selected photo at Fit. No full grid or editing panels are needed. The owner selected one workspace with collapsible editing panels for M1; no full grid is included. Neutral dark surfaces, strong typography, predictable focus and accessible controls keep attention on the photo. Native dialogs and platform shortcuts should fit each OS even if controls are custom drawn.

Crop should feel familiar to Lightroom users: direct handles, aspect-ratio locking, a straightening angle, and visible confirmation/cancellation. The owner requires free crop control with draggable sides and proportional Option resizing about the fixed center. Straightening preserves composition with only necessary trimming. M1 includes zoom/pan, numeric percentage settings and Fit. Exports strip optional metadata by default and offer a Keep metadata setting. The initial tool order and workspace are chef's choices made by the owner. Later customization should allow hiding/reordering panels and adding tools, while preserving one easy reset to defaults.

No Map, Book, Slideshow, Print, Web, or Publish Services modules are planned. An optional Instagram-oriented export module is an example of the extension boundary, not a commitment to build publishing services. Cloud sync, accounts, built-in AI chat, generative editing, and a plugin marketplace are not necessary to prove the four pillars. Agent-native means structured access to the actual application, not a mandatory model provider or chat sidebar.

The owner reaffirmed full programmability and the small host/module design in D19–D21. Exposure, white balance, clone strokes and mask operations must have complete APIs whenever implemented; this does not move future tools into M1. Prefer optional features where useful, but measure avoided initialization, allocations and background work before assuming plugin binaries improve speed. The engineering recommendation is lightweight linked built-ins with lazy resources, plus a later external loader regardless of the built-in packaging choice. See [module responsibilities, lifecycle and acceptance](design/modules-and-api.md).

## Milestones and exit gates

These are work slices within v0, not releases or compatibility promises. Sequence after M1 is a proposal for owner review.

| Milestone | Outcome | Evidence required to finish |
| --- | --- | --- |
| M0 — narrow bootstrap decisions/probes | Select the skeleton platform scope, license disposition, UI/decoder and minimal input/display contract | TASK-035 product gate plus focused window/image probes; no dependency on crop, RAW, catalog or MCP decisions |
| S0 — first end-to-end build | Cross-platform image-loading skeleton and development/agent tooling | [Skeleton acceptance](specs/bootstrap.md#acceptance): reproducible setup/check/build commands, three-platform build/package evidence and native M4 launch/load evidence, logs/state/captures, M4 baseline; TASK-063 closes the gate |
| M1 — one-image editor | Extend the skeleton with referenced JPEG catalog, geometry, persistence, undo/redo, export, manual Locate, JSON commands, local IPC and MCP | M1 decision gate TASK-064 and geometry/color proof TASK-004; full [editor acceptance journey](specs/single-image.md#acceptance-journey) on M4, including live conflicts and moved-source recovery |
| M1 portability verification | Repeat the added editor journey on Windows and Linux | TASK-019 validates new editing/persistence/IPC behavior; S0's successful load-only checks do not prove these later features |
| M2 — useful small library | Multi-image import, virtualized grid/filmstrip, indexed filtering, preview cache; broader source recovery and batch automation | Large synthetic catalog queries plus real image browsing; folder relocation/duplicate cases; cancel/retry and batch-edit behavior |
| M3 — basic tonal editing | Exposure, white balance, contrast, highlights/shadows, whites/blacks, saturation/vibrance; PNG | Explicit mathematical/visual specs for each tool; reference images; preview/export consistency; bounded processing |
| M4 — first trustworthy RAW support | Nikon Z6 and Fujifilm X100VI in tested recording modes | Real sample matrix; decoder and color/demosaic quality evaluation; memory and latency measurements; honest unsupported-mode reporting |
| M5 — richer development and extensions | Texture, clarity, dehaze; customizable/optional tools and a loaded external module proof | Image-quality fixtures, measured activation costs, missing-module edit preservation, and an independently authored module loaded without host source changes through documented APIs; TASK-033/067 scope the proof |

The camera models are now known, so RAW compatibility research can happen before M4; recording-mode samples and benchmarks remain outstanding. This must not imply RAW editing is implemented. If RAW is the earliest daily-use requirement, move the M4 work ahead of the full M3 tool set after M1. Do not commit to calendar estimates before the relevant framework and image-quality probes produce evidence.

## Architectural bets worth making now

- Originals plus an edit recipe; never repeatedly bake edits back into source pixels.
- One operation registry/service used by UI and automation. The core owns shared invariants and recipe/history transactions; modules own feature validation and effect algorithms through host APIs.
- Metadata queries and disposable previews separate from original storage.
- Bounded background work and progressive previews; visible interaction takes priority over indexing.
- A fixed, modular processing pipeline initially. Built-ins exercise host contracts before external loading; use optionality and lazy initialization where useful, and measure binary splitting rather than assume a speedup.
- Color meaning, coordinate systems, and persistence semantics explicit from the first image.

See [architecture](design/architecture.md) for the proposed boundaries and [performance](specs/performance.md) for measurable targets. These are design choices to evaluate, not claims that selecting Rust or a GPU guarantees speed.

## Important risks and decision gates

| Risk | Early way to resolve it |
| --- | --- |
| UI framework requires too much custom desktop infrastructure | Test crop input, keyboard focus, text/IME, accessibility, high DPI, menus/dialogs, GPU sharing, and packaging before building panels |
| Tagged photos or monitor output are wrong | Test color conversion and presentation end to end, including a wide-gamut display and avoiding double conversion |
| RAW opens but looks poor | Separate container decoding from demosaic, profiles, white balance, highlight handling, and an intentional baseline rendering |
| Large catalogs feel slow despite fast shaders | Benchmark query paging, thumbnails, disk latency, scheduling, cold startup, and cancellation separately |
| Extensibility makes the first app complicated | Start with built-in modules and command schemas; add one real external extension before designing an ecosystem |
| UI and agents overwrite each other's changes | Shared transactions, revision preconditions, explicit job state, and a single catalog owner |

## Planning and implementation handoff

The [feature matrix](features.md) is authoritative for implementation state. [User documentation](user-guide.md) explains the intended experience and currently available behavior. [Decision notes](decisions.md) distinguish owner requirements from proposed defaults.

The [product decision plan](../tasks/product-decisions.json) and [implementation plan](../tasks/implementation.json) separate owner choices from engineering work. The implementation plan details S0 bootstrapping and retains the subsequent M1 tasks. [Task sequencing](task-planning.md) explains preserved IDs, external decision gates and the historical foundation snapshot. Later features remain at roadmap level until selected. Each slice begins with a Markdown specification, then validated JSON tasks using Create Tasks. No implementation task is complete merely because this plan exists.

## Owner workflow priorities

Prioritize measured speed, avoiding unused-tool bloat and clear modern export defaults. M2 design should address filtering, coherent tagging/collections, editing a lazily loaded shoot within a full catalog, and multi-selection/stacking. Research bracket/panorama identification separately from merging. These are recorded pain points; feature solutions and scope still require decisions. See [the owner answer](decisions.md#owner-workflow-priorities--2026-09-19).
