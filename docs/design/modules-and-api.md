# Tool modules and the shared core

Status: planned for M3, on top of the working history foundation and transforms.

An operation is semantic and programmable: set one pixel, rotate, set crop parameters, restore history, select a preview or change view state. Scripts never simulate pointer movement. Exposure, masks, clone strokes and lifecycle actions inherit this rule when they arrive.

## Module contract

| Contribution | Meaning |
| --- | --- |
| Identity | Stable provider, effect and action IDs plus an internal payload format marker |
| API | Input and result schemas, ranges, units, coordinate space, defaults, availability and structured errors |
| Controls | Semantic numbers, colors, enums, actions and grouping, plus an optional canvas interaction adapter |
| Validation | Tool-specific parameter and state validation with the same result for every caller |
| Processing | Deterministic evaluation of immutable layer payloads with declared input/output geometry and color, resource needs and cancellation |
| State | Inspectable current parameters and capability information |

The GUI chooses layout, styling and focus behavior and binds controls or canvas gestures to declared actions. A control never has a hidden GUI-only mutation. Every registered action appears through common discovery, with explicit treatment when the GUI cannot display a control type.

The host alone owns asset identity, shared invariants, atomic recipe and history commits, revisions, undo/redo, request deduplication, job scheduling and notifications. Module code cannot write catalog tables or keep a separate authoritative history. Modules receive snapshots for processing and call host services for commits; preview, undo and restore never ask a module to reverse pixels.

## Migration proof

Move the pixel operation into a module with its action and control schema, then adapt the transform handlers to the same interface. Preserve effect IDs, axis conventions, operation ordering and data meaning: catalogs saved before M3 and every history snapshot must evaluate identically. The proof is a working registered tool with generated controls, headless calls and live GUI/API parity. It does not need a separate binary.

## Missing effects

Retain unrecognized or unsupported payloads losslessly. A missing or disabled provider reports the affected snapshots and blocks trustworthy rendering and export rather than omitting its effect. A cached preview may only be shown with an explicit stale or unavailable label. Re-enabling a compatible provider restores evaluation without rewriting history. Removing an edit from a recipe is a separate explicit undoable action. Use internal format checks and an explicit tested conversion only when needed; v0 has no public compatibility framework or stable ABI.

## Resources and external loading

Registration is cheap and expensive resources initialize on demand. Start with linked built-ins. Hiding controls is a UI preference, not effect removal, and persisted processing order never depends on panel order.

External loading remains required later: a separately authored tool must load without editing host source and expose its actions through the same APIs. Before implementing that loader, select a use case, measure startup, first-use, memory and idle costs, and define trust, dependencies, cancellation and packaging. Native binaries, workers or sandboxed runtimes are all still options; a marketplace and hot unload are not requirements. Later operations such as exposure, white balance, masks, clone strokes or presets must carry enough structured data to reproduce their results. Keep geometry and color stages explicit and do not prebuild a generalized graph.

## Acceptance

Descriptor validation rejects duplicate identities, missing handlers, invalid controls and unavailable providers explicitly. Pixel and transform actions invoked through generic controls and an independent API client produce identical stacks, history, pixels and errors. Fixtures saved before module integration reopen with all IDs and undo/restore paths intact. Registration and first-use resources are measured separately. Real native M4 control layout, keyboard behavior and pixels are inspected with correlated state.
