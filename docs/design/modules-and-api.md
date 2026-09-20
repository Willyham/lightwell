# Tool modules and the shared core

Status: **planned M3**, following the working history foundation and basic transforms. The owner requested a module interface declaring actions, API and controls, with the pixel editor implemented through it. See [the roadmap](history-first-roadmap.md).

## Delivery order

M1 implements concrete pixel actions and the authoritative layer/history service. M2 proves that the same model supports rotation/reflection. M3 introduces module registration and control descriptions around those working effects. M4 adds the crop tool through that interface.

An operation is semantic and programmable: set one pixel, rotate, set crop parameters, restore history, select a preview or change view state. Scripts never need to simulate pointer movement. Exposure, masks, clone strokes and lifecycle actions inherit this rule when introduced; they remain later scope.

## Module contract

| Module contribution | Required meaning |
| --- | --- |
| Identity | Stable provider/effect/action IDs and an internal payload format marker |
| API | Input/result schemas, ranges, units, coordinate space, defaults, availability and structured errors |
| Controls | Semantic numbers, colors, enums, actions and grouping, plus optional canvas interaction description/adapter |
| Validation | Tool-specific parameter and state validation with the same result for every caller |
| Processing | Deterministic evaluation of immutable layer payloads with declared input/output geometry/color, resource needs and cancellation |
| State | Inspectable current parameters and capability/availability information |

The GUI chooses layout, visual styling and focus behavior. It binds controls or canvas gestures to declared actions. A control must not have a hidden GUI-only mutation. Every registered action appears through common discovery/API, with explicit treatment if the current GUI cannot display a control type.

Modules produce validated edit changes. The host alone owns asset identity, shared invariants, atomic recipe/history commits, revisions, undo/redo, request deduplication, job scheduling and notifications. Module code cannot write catalog tables or maintain a separate authoritative history.

## Pixel module proof

Move the existing x/y/RGB pixel operation into a module with its action and control schema. Preserve the effect ID and data meaning; pre-module catalogs and every saved history snapshot must evaluate identically. The first pixel tool is the reference for registration, validation, control rendering and API dispatch.

Adapt M2 transform handlers to the same interface while preserving IDs, axis conventions and operation ordering. The proof is a working registered tool with generated controls, headless calls and live GUI/API parity. It does not require a separate binary.

## History and missing effects

A layer stores effect identity and parameters, while each history entry stores a complete immutable stack. Modules receive snapshots for processing and call host services for commits. Preview/undo/restore share this model rather than asking a module to reverse pixels.

Retain unrecognized/unsupported payloads losslessly. A missing or disabled provider reports affected snapshots and blocks trustworthy rendering/export rather than omitting its effect. A cached preview can only be shown with an explicit stale/unavailable label. Re-enabling a compatible provider restores evaluation without rewriting history. Removing an edit from a recipe is a separate explicit undoable action.

Use internal format checks and an explicit tested conversion only when needed. v0 does not require a public compatibility framework or stable ABI.

## Resources and external loading

Keep registration cheap and initialize expensive resources on demand. Begin with linked built-ins. Hiding controls is a UI preference, not implicit effect removal or processor disablement. Avoid tying persisted processing order to panel order.

Later external module loading remains required: a separately authored tool must load without editing host source and expose actions through the same APIs. Select a use case, measure startup/first-use/memory/idle costs and define trust, dependencies, cancellation and packaging before implementing that loader. Native binaries, workers or sandboxed runtimes remain options; a marketplace and hot unload are not current requirements.

Module operations added later—exposure, white balance, masks, clone strokes or presets—must specify enough structured data to reproduce results. Keep geometry/color stages explicit; do not prebuild a generalized graph in M3.

## Acceptance

Validate descriptor consistency and reject duplicate identities, missing handlers, invalid controls and unavailable providers explicitly. Invoke pixel and transform actions through generic controls and an independent API client, comparing complete stacks, history, pixels and error results. Reopen fixtures saved before module integration and verify all IDs and undo/restore paths survive.

Measure registration and first-use resources separately. Inspect real native M4 control layout, keyboard behavior and pixels with correlated state. The crop module is the next consumer of this contract.
