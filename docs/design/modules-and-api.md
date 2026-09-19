# Programmable operations and a small module host

Status: **owner requirements accepted on 2026-09-19; implementation design proposed**. This document records D19–D21 in [decisions](../decisions.md). The current S0 viewer has no editor API or plugin loader. M1 establishes the shared host and built-in tool boundaries; external loading follows the scoped extension work. This planning update does not implement those capabilities or add future editing tools to M1.

## Accepted requirements

- Every application operation must be available through an API to code and agents. This includes every photo edit, whether bundled or supplied by an external module. Exposure, white balance, clone operations and mask creation/editing are explicit examples; their API requirement does not itself schedule those features.
- The application is a shell and module host with a small set of shared services. The core provides non-destructive state, change application, history and undo/redo; modules supply feature behavior through those services.
- Prefer optional modules where users benefit from choosing what runs. Avoid turning fundamental services into separately loaded plugins without a meaningful benefit. External modules must still be loadable and able to use core APIs even if built-in functionality stays linked into the application.

## What an operation means

Expose semantic actions and state, not a requirement to simulate mouse movements. A script supplies a crop rectangle, mask path or clone stroke directly. It must not need an open panel, active pointer gesture or GUI focus to perform the operation. UI adapters may keep pointer capture and widget focus locally; the resulting application behavior must have an API equivalent.

Coverage includes catalog and source operations, all edit parameters and tool actions, history, previews, export, selection, zoom/pan/Fit, settings, and module discovery/enablement when each capability exists. The UI may invoke typed services in process; CLI, local IPC and MCP adapt the same operation registry. JSON is a transport representation, not the inner pixel-processing interface. Headless clients use the same feature implementations, subject to explicit capabilities and required render resources.

Every operation definition supplies input/result schemas, units, coordinate spaces, defaults, validation, errors, availability, state queries and notifications as appropriate. Durable mutations use the shared revision, request/retry, transaction and history rules. Session/settings operations declare their persistence and undo policy rather than pretending every action is an image edit. Discovery distinguishes available operations from disabled, missing or unsupported providers. A module cannot expose a GUI-only feature or maintain a separate private editing API.

For future tools, the feature specification must cover enough structured data to reproduce the action:

| Example | Required programmable behavior when implemented |
| --- | --- |
| Exposure / white balance | Read and set values with defined units, ranges, color domain and processing order; use the same validator and rendering as the UI |
| Masks | Create, inspect, modify and remove identified masks; specify shape/stroke coordinates, feathering, combination rules and attachment to edits |
| Clone tool | Specify source/target coordinates, stroke samples and relevant brush parameters; preserve referenced source identity and ordering in reproducible recipe data |
| Preset | Inspect and apply its parameter changes as a defined host transaction with meaningful undo behavior |
| Optional module | Inspect availability/dependencies, enable/disable it, and observe the result through the same host lifecycle API used by settings |

These examples define coverage, not command names or a runnable schema. Randomness, external assets or other inputs affecting an edit must be captured or explicitly referenced and verified; replay must not depend on hidden UI state. Large stroke/mask payloads need bounded data transfer and storage, not unbounded protocol frames.

## Host and module responsibilities

| Boundary | Responsibility |
| --- | --- |
| Desktop shell | Window, navigation and controls; turns gestures into shared operations and displays authoritative state |
| Core host | Operation/provider registration, capability discovery, asset identity, recipe ownership, atomic change/history commits, revisions, undo/redo and persistence access |
| Shared engine services | Color/coordinate contracts, ordered render scheduling, immutable snapshots, bounded jobs/caches, source protection and safe output publication |
| Feature modules | Parameter definitions, feature-specific validation, processing implementations and optional UI contributions; use host services for changes and jobs |
| Transport adapters | Expose the same operations to programs/agents through JSON, IPC and MCP without a second implementation of feature rules |

The core owns **how a change becomes authoritative**; an exposure module owns **what exposure means mathematically**. For example, a proposed exposure operation validates EV in the module, produces a recipe change against an expected revision, and submits it to the host. The host atomically stores the new recipe and history and schedules a preview. Rendering calls the registered exposure implementation on an immutable snapshot. Undo restores the previous recipe through the host; it does not ask the module to apply an inverse pixel transform.

The [M1 history contract](../specs/edit-history.md) requires every committed image change to enter the host-owned action log with semantic action metadata and a complete reconstructable result. The host provides entry listing/inspection, historical preview and restore as well as undo/redo. This applies to future tools and external modules when introduced.

Module code must not write catalog tables, source pixels or a separate authoritative undo stack. UI and automation use the same module validators and processing path. A provider describes its identifier, operations, parameters, recipe data and required render stage/dependencies. The host enforces shared invariants and rejects invalid/unsupported changes. Built-ins exercise this boundary first. These responsibilities need not become one crate or shared library each.

Keep a fixed, documented processing order initially. Panel order and enablement must not silently reorder exposure, white balance, geometry or other stages. Later pixel-processing providers require explicit formats, color meaning, region/halo needs and resource/cancellation contracts. Do not build a generalized node graph, stable public ABI or compatibility framework in v0. Internal data markers and explicit incompatibility errors still protect saved work.

## Optionality and performance

Distinguish four choices: a logical code module, a user-enabled feature, lazily initialized resources, and a separately distributed/loaded binary. None implies the others. Hiding a panel is a layout preference and does not disable its processor. Compile-time feature selection alone is not user runtime enablement.

**Engineering recommendation:** retain mandatory host services and a useful default editor; implement small built-in tools behind the common contracts, normally linked into the app. Keep registration cheap and initialize expensive resources when needed. Prefer independently optional presets, workflow/export integrations and substantial decoder/resource groups. Consider separate binary loading only where dependency size, startup work, resident resources or external authorship justify it.

Avoiding initialization, allocations and background work can help startup and idle resource use; a Lightwell speedup remains unmeasured. Dynamic libraries can also introduce loader costs, and dynamic dispatch can inhibit inlining. Apple's [dynamic-library guidance](https://developer.apple.com/library/archive/documentation/DeveloperTools/Conceptual/DynamicLibraries/100-Articles/DynamicLibraryDesignGuidelines.html) distinguishes launch-linked libraries from runtime loading and warns about poor designs; the [Rust Book](https://doc.rust-lang.org/stable/book/ch18-02-trait-objects.html) explains dispatch overhead. These support measuring the tradeoff, not a choice of plugin runtime or a performance claim for this app.

Exposure is a candidate for a lightweight built-in module; splitting its control into a separate binary has no established payoff. If Nikon and Fujifilm support share a decoder library, disabling Nikon alone may save little; choose actual dependency/resource boundaries after measurement. An Instagram-oriented export module is an example of optional workflow integration, not a commitment to build a publishing service. Presets can be optional without making the host's transaction or history service optional.

Compare minimal/default configurations, optional modules disabled, enabled but unused, and first active use. Record startup, resident/peak memory, CPU/GPU allocations, idle work and first-use latency on the M4, using identical images and output correctness checks. Include the cost merely shifted from startup to first use. If a split has no material benefit, keep the simpler linked implementation and its API boundary. See the [measurement plan](../specs/performance.md#module-activation-measurements).

## Disabling modules without losing edits

Proposed lifecycle rules, to finalize with the first optional-module use case:

- Discover installed providers and their dependencies from bounded metadata where possible; do not execute every optional module merely to draw its listing. Disabled modules do not initialize processors or start background jobs. Static linkage does not promise that every byte of their code is absent from memory.
- Keep required host services available. Identify mandatory/default modules explicitly, and report dependencies preventing disablement. UI and API must give the same result.
- Deactivation must account for in-flight jobs and immutable render snapshots. A restart-required transition is acceptable if documented; hot unload is not a requirement. Never unload code or resources still in use.
- Disabling/uninstalling a provider does not delete its recipe data, history or asset references. Preserve unknown payloads losslessly and identify affected images. Undo/redo must preserve their data even when rendering is unavailable.
- If a recipe needs an unavailable processor, show an explicit unavailable-effect state and block final export rather than silently omit it. A retained cached preview may be shown only as stale/unverified. Re-enabling a compatible provider restores evaluation without rewriting the user's edits.

Removing an effect from an image is a separate, explicit, undoable recipe operation; it is not an implicit side effect of hiding or disabling its module. This preserves the existing source/recipe protection requirement.

## Delivery and acceptance

| Stage | Planned evidence |
| --- | --- |
| S0 | Existing shared open service and developer harness; no new plugin runtime or production MCP prerequisite |
| M1 | Built-in geometry/decoder providers exercise registration, module validation/processing and host-owned history. Every M1 action is mapped to the shared service, JSON and MCP, with live GUI/headless parity |
| Later optional modules | Measurement-backed loading/enablement choices, lifecycle API, dependency handling and missing-provider recovery with existing edits |
| External extension proof | A separately authored module is installed/loaded and invokes documented host APIs without changes to host source; its applicable operations are available through UI and programmatic adapters, with disable/re-enable and failure evidence |

For M1, TASK-007/008/009/011 establish the boundary, TASK-014/021 expose it, and TASK-016 verifies an operation coverage matrix and equivalent recipes/history/results. No masks, clone, tonal tools, optional-module manager or loader are added to M1 by this decision. The API requirement applies to every new capability when it is introduced.

Product TASK-033 chooses the first external use case and its user-visible lifecycle; TASK-034 sets meaningful performance priorities. Implementation TASK-067 then uses M1 experience to benchmark activation and write a concrete loader/proof specification and validated implementation tasks. External loading is required even if measurements favor linked built-ins. API access by an external script, by itself, does not complete the loaded-module proof. The current M5 placement is retained; moving it earlier is a separate prioritization decision.

Open decisions: the first extension's purpose; which features are independently optional; acceptable startup/first-use tradeoffs; runtime/package format, trust/isolation and UI contribution mechanism. WASM, native workers and declarative controls are options to evaluate against that use case, not accepted selections. Cross-platform loader behavior and resource limits must be included in its specification. No marketplace, hosted service or unrestricted native pixel ABI is authorized by this plan.
