# Task planning and milestone order

Task plans use independent IDs and follow the history-first delivery sequence. S0 is accepted; M1 and M2 are completed and locally verified, while M3 and later work remain pending.

## Local plans

Each active JSON plan listed in [the index](../tasks/README.md) starts with `TASK-001`. IDs are unique only inside that plan, so the same ID can occur in every file. Read tasks top to bottom in dependency order. Dependencies refer only to earlier tasks in the same file; independent tasks share an execution wave. Do not serialize unrelated work just to make every wave contain one task.

A plan contains its own behavior, context, acceptance and tests. Link Markdown specifications or code for context, and describe required existing capabilities by name. Do not link another task plan, place its ID in prose, or add a checker dependency on its completion. Product decisions become authoritative through the recorded decision/specification, not a cross-file ID lookup.

Milestone sequencing belongs in the [history-first roadmap](design/history-first-roadmap.md). M1 establishes a usable history/pixel editor; M2 adds transforms; M3 introduces tool modules; M4 delivers the crop module. Each later plan begins from the named working capability.

## Current plans

| File | Tasks | Waves | Starting state |
| --- | --- | --- | --- |
| [S0](../tasks/implementation-s0.json) | 37 | 13 | Accepted milestone; follow-ups remain unfinished |
| [M1 history](../tasks/implementation-m1-history.json) | 9 | 7 | Completed and locally verified |
| [M2 transforms](../tasks/implementation-m2-transforms.json) | 5 | 5 | Completed and locally verified |
| [M3 modules](../tasks/implementation-m3-modules.json) | 6 | 5 | Pending activation after transforms |
| [M4 crop](../tasks/implementation-m4-crop.json) | 6 | 6 | Pending activation after modules |
| [Editor follow-ups](../tasks/implementation-editor-followups.json) | 6 | 4 | Planned export, Locate, MCP and verification |
| [Later extensions](../tasks/implementation-extensions.json) | 1 | 1 | Pending a selected use case and working module editor |
| [Product decisions](../tasks/product-decisions.json) | 14 | 1 | Existing decisions and unresolved questions retained |
| [Lightroom research](../tasks/research-lightroom.json) | 1 | 1 | Completed |
| [darktable research](../tasks/research-darktable.json) | 1 | 1 | Completed; see its research verification |

## Maintaining plans

Preserve local IDs during routine updates. Each status describes actual work, and completed tasks retain the evidence needed to support their status. Keep only current plans with concise context and no planning-change logs. A planning edit does not complete implementation.

## Verification

Use the Create Tasks schema without extending it with cross-file dependency fields. Validate every changed plan and synchronize its derived waves. Run `cargo xtask check`, which validates every active top-level JSON file, unique plan names, contiguous ordered local IDs, local dependency status and Markdown links.

The checker deliberately allows matching task IDs in different plans. It rejects duplicate IDs inside a plan, gaps/out-of-order numbering, dependencies on later/unknown tasks, stale waves and file links into another task plan. Milestone sequencing and owner decisions are assessed from the named acceptance outcomes and specifications.

The S0 advisory policy explicitly reads the S0 plan: its paste and ttf-parser follow-ups are local TASK-016 and TASK-017. Expiry/version enforcement remains in place. This code lookup has no bearing on another plan's TASK-016 or TASK-017.

## Proportionate planning

Use Markdown design and task decomposition for substantial coordinated changes or when requested. Standalone research, diagnostics, reviews and small contained edits do not require creating a task plan unless requested or already assigned from one. Maintain statuses with actual implementation evidence, and keep future scope explicit.
