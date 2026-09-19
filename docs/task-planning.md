# Task plans and sequencing

Status: **maintained scaffold executing; S0 product and stack gates complete**. The latest owner instruction makes S0, a cross-platform image-loading skeleton, the first end-to-end build. Editing remains the subsequent M1 milestone.

## Active plans

- [Product decisions](../tasks/product-decisions.json): unresolved choices, research/interview questions and owner decision records. A task completes when its answer and consequences are documented, not when software is built.
- [Implementation](../tasks/implementation.json): technical probes, repository setup, tooling, linting, builds, image loading, diagnostics, verification and packaging; followed by the retained M1 editor work.
- [Original foundation snapshot](../tasks/archive/v0-foundation.json): historical record only. Do not execute or update this archived plan.

IDs are globally unique across the two active files. TASK-001 moves to the product plan; TASK-002 through TASK-022 retain their IDs in implementation. New IDs start at TASK-023. The earlier broad setup task TASK-006 now creates just the workspace; environment setup, checks, build orchestration and CI have separate tasks. Existing tasks are narrowed/resequenced rather than falsely completed. No implementation task is completed by this planning update.

## Current plan size and starting points

The product plan has 13 tasks in one dependency wave: TASK-001 is the ongoing interview and TASK-023–TASK-034 are available decision tasks. Prioritize TASK-023–TASK-025 for S0; later product choices can wait.

The implementation plan has 51 tasks in 23 derived waves: 32 required S0 tasks, one optional Linux VM task and 18 subsequent M1 tasks. The original ready tasks were TASK-002 (fixtures), TASK-035 (inspect the S0 decision gate) and TASK-037 (repository conventions). TASK-002/003/005/006/035/036/037/038 are complete. Product TASK-023–025 are complete. The maintained workspace, command runner, image loader and evidence slice run on macOS; subsequent tasks remain open until their full criteria pass.  A ready gate is not a completed decision; its missing external answers must still be resolved. See the [execution evidence](engineering/s0-probe-results.md) for completed work and limitations.

## Milestone boundaries

| Work | Implementation tasks | Completion rule |
| --- | --- | --- |
| Fixtures, narrow framework/image probes and stack choice | TASK-002, TASK-003, TASK-005, TASK-035, TASK-036 | Supports loading/display only; does not wait for editor decisions |
| Repository, workspace, toolchain and build/check setup | TASK-006, TASK-037–TASK-042, TASK-052 | Repeatable local and three-OS build/check workflow |
| Window, diagnostics, loading and Fit display | TASK-043–TASK-048 | Read-only image viewer with truthful states and shared open path |
| Agent captures, correctness, smoke tests and CI evidence | TASK-049–TASK-053 | Observable actual rendering, retained results and failure evidence |
| Packages, native checks, measurements and playbook | TASK-054–TASK-062 | Platform-specific evidence; TASK-060 Linux VM work is optional and not on the S0 gate |
| S0 acceptance | TASK-063 | Three-platform launch/load gate; no editing tools |
| M1 specification gate and editor work | TASK-064; TASK-004; TASK-007–TASK-022 | Retains catalog, geometry, undo, export, live agents and Locate after S0 |

TASK-052/TASK-053 appear in two rows because CI connects the build and evidence workflows. See JSON dependencies for the authoritative order, not numeric IDs or table order.

## Dependencies across separate files

The Create Tasks schema validates dependencies within one file. Do not invent cross-file IDs in `dependencies`. Link external decisions with a file context link and a task locator. Two explicit implementation gates bridge the plans:

- **TASK-035:** checks the S0 platform matrix (TASK-023), license disposition (TASK-024), and minimal shell scope/visual direction (TASK-025). Its acceptance requires those product tasks completed and recorded. Fixture preparation and technical probes can proceed independently.
- **TASK-064:** checks M1 workspace/geometry, state/history, export and live-agent decisions (TASK-026–TASK-030), and requires S0 acceptance. It does not make later RAW, storage or extension decisions a skeleton prerequisite.

Execution waves are topological views within each plan, not a global schedule or an instruction to dispatch agents. A ready gate may inspect its missing inputs; it must not mark itself complete while its external decisions remain unresolved. When completing either gate, check the current product JSON statuses and decision notes, and record the referenced IDs. A future task-plan check should verify these gate conditions too.

Product tasks marked ready are available for short interview/research rounds, not a demand to answer everything before coding. Their `extra_context` states when each answer is needed. TASK-001 keeps the existing interview in progress without making the entire interview a dependency of the first window.

## Detailed bootstrap sequence

First gather the S0-specific decisions and fixtures while trying the minimal photo surface. Record the selected stack; establish repository conventions, pinned tools and a workspace; add the task runner, linting, dependency checks and isolated runtime paths. Build a window, diagnostics and bounded decode independently where their prerequisites allow, then connect Fit rendering and file opening.

Add deterministic state/readiness controls and frame capture through the real app. Use them in correctness and process-level smoke tests. Package the same app for each OS, verify it in a desktop session, collect M4 baseline measurements, and publish a developer/agent playbook with real commands. Only then close S0 and enter the editor gate.

The current documents specify future command outcomes; task `test_strategy.commands` arrays are populated as real commands are implemented. Task outputs must add runnable commands and evidence paths when implemented.
