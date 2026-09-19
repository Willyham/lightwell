# Task plans and sequencing

**Current owner override (2026-09-19):** manual Windows/Linux desktop checks and manual license reviews are deferred. S0 currently requires native M4 evidence and automated portable checks; deferred checks are not passing results. See [hardening scope](engineering/s0-hardening.md).

Status: **maintained scaffold executing; S0 product and stack gates complete**. The latest owner instruction makes S0, a cross-platform image-loading skeleton, the first end-to-end build. Editing remains the subsequent M1 milestone.

## Active plans

- [Product decisions](../tasks/product-decisions.json): unresolved choices, research/interview questions and owner decision records. A task completes when its answer and consequences are documented, not when software is built.
- [Implementation](../tasks/implementation.json): technical probes, repository setup, tooling, linting, builds, image loading, diagnostics, verification and packaging; followed by the retained M1 editor work.
- [Original foundation snapshot](../tasks/archive/v0-foundation.json): historical record only. Do not execute or update this archived plan.

IDs are globally unique across the two active files. TASK-001 moves to the product plan; TASK-002 through TASK-022 retain their IDs in implementation. New IDs start at TASK-023. The earlier broad setup task TASK-006 now creates just the workspace; environment setup, checks, build orchestration and CI have separate tasks. Existing tasks are narrowed/resequenced rather than falsely completed. No implementation task is completed by this planning update.

## Current plan size and starting points

The product plan has 13 tasks in one dependency wave: TASK-001 is the ongoing interview, TASK-023–TASK-025 are completed S0 decisions, and TASK-026–TASK-030 are completed M1 decisions, and TASK-031–TASK-034 remain ready for their scoped interview/research work. D19–D21 already settle full programmability and the small module-host direction; TASK-033/034 retain use-case/lifecycle and performance questions, not whether to support external modules.

The implementation plan has 55 tasks in 23 derived waves: 32 originally required S0 tasks, one optional Linux VM task, 18 subsequent M1 tasks, two dependency-exception follow-ups (TASK-065/066), one later module measurement/planning task (TASK-067), and the completed real-JPEG performance investigation (TASK-068). The owner override above changes the current S0 gate. The original ready tasks TASK-002 (fixtures), TASK-035 (S0 decision gate) and TASK-037 (repository conventions) are completed. Current statuses and dependencies in the JSON are authoritative for ongoing scaffold work. TASK-067 is pending after the verified M1 handoff in TASK-018; this architecture update makes no new implementation task immediately runnable and marks no existing task complete. See the [execution evidence](engineering/s0-probe-results.md) for completed work and limitations.

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
| Module activation and external-loader planning | TASK-067 after TASK-018 | Measure the selected use case and produce scoped loader/proof implementation tasks; external loading itself remains undelivered |

TASK-052/TASK-053 appear in two rows because CI connects the build and evidence workflows. See JSON dependencies for the authoritative order, not numeric IDs or table order.

## Dependencies across separate files

The Create Tasks schema validates dependencies within one file. Do not invent cross-file IDs in `dependencies`. Link external decisions with a file context link and a task locator. Two milestone gates and one later planning prerequisite bridge the plans:

- **TASK-035:** checks the S0 platform matrix (TASK-023), license disposition (TASK-024), and minimal shell scope/visual direction (TASK-025). Its acceptance requires those product tasks completed and recorded. Fixture preparation and technical probes can proceed independently.
- **TASK-064:** checks M1 workspace/geometry, state/history, export and live-agent decisions (TASK-026–TASK-030), and requires S0 acceptance. It does not make later RAW, storage or extension decisions a skeleton prerequisite.
- **TASK-067:** checks that product TASK-033 has selected the external use case/lifecycle before dependent prototype/design work; consults TASK-034 priorities without inventing settled budgets. Its local dependency is the verified M1 host/API handoff. Completion must record the external input and validated follow-on tasks for actual module loading. It cannot complete the external feature merely by documenting an API client.

Execution waves are topological views within each plan, not a global schedule or an instruction to dispatch agents. A ready gate may inspect its missing inputs; it must not mark itself complete while its external decisions remain unresolved. Check current product JSON statuses and decision notes and record referenced IDs when completing the gates or TASK-067. The repository checker covers TASK-035/064; TASK-067's external input currently requires inspection and must be covered when the later loader tasks/checks are implemented.

The [module/API contract](design/modules-and-api.md) is binding context for new feature planning: every operation needs a common API, module-owned feature semantics use core change/history services, and UI/programmatic parity is an acceptance criterion. M1 TASK-007/008/009/011 implement that boundary and TASK-014/021/016 expose and verify it. Later feature plans must carry these requirements into exposure, white balance, clone/mask tools and external modules as those features are scoped; those examples do not add tools to M1.

Product tasks marked ready are available for short interview/research rounds, not a demand to answer everything before coding. Their `extra_context` states when each answer is needed. TASK-001 keeps the existing interview in progress without making the entire interview a dependency of the first window.

## Detailed bootstrap sequence

First gather the S0-specific decisions and fixtures while trying the minimal photo surface. Record the selected stack; establish repository conventions, pinned tools and a workspace; add the task runner, linting, dependency checks and isolated runtime paths. Build a window, diagnostics and bounded decode independently where their prerequisites allow, then connect Fit rendering and file opening.

Add deterministic state/readiness controls and frame capture through the real app. Use them in correctness and process-level smoke tests. Package the same app for each OS, verify it in a desktop session, collect M4 baseline measurements, and publish a developer/agent playbook with real commands. Only then close S0 and enter the editor gate.

The current documents specify future command outcomes; task `test_strategy.commands` arrays are populated as real commands are implemented. Task outputs must add runnable commands and evidence paths when implemented.

## Current hardening handoff

TASK-042–046, TASK-048–051, TASK-054 and TASK-061 now have local completion evidence in [hardening results](engineering/s0-hardening-results.md). TASK-047 and TASK-057 are also complete after the owner confirmed manual JPEG opening, with automated selection limits kept explicit. TASK-052/053 retain fresh hosted execution; prior hosted results remain historical evidence. The packaging and measurement dependencies now express actual build outputs rather than waiting on manual picker work.

TASK-041 and TASK-058/059 are deferred and removed from current S0 gates. Existing automated dependency policy and TASK-065/066 follow-ups remain. M1 TASK-026–030 now have [owner-accepted decisions](design/m1-decisions.md); the product round is complete. S0 and M1 gates remain incomplete.
