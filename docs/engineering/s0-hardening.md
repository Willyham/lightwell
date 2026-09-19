# S0 hardening and revised acceptance

Owner instruction, 2026-09-19: defer manual Windows/Linux checks and license/review work for now. Preserve their task IDs and unfinished evidence; neither is a current S0 prerequisite. Keep automated portable builds and functional checks. macOS on the available M4 is the current native acceptance target; minimum-version and other desktop claims remain unverified.

## Scope and behavior

Complete TASK-042–051, refresh TASK-054/057 and measure TASK-061. Finish the associated CI/playbook gate evidence where available. Existing source-preservation, JPEG subset, Fit-only scope and bounded scheduling remain unchanged. No M1 editor implementation is authorized by this hardening slice; the separately requested product decision tasks must record actual answers before TASK-064 and the geometry/color experiment.

Use typed errors at the shared image boundary. Resolve native per-user configuration/cache/log locations without creating unused state; an explicit isolated data root redirects all three. Ordinary viewing must work if diagnostics cannot be written. Opt-in evidence directories must be new, bounded and fail explicitly on write errors.

Write incremental JSONL diagnostics on a bounded background channel, carrying run/build/request/generation identity and timings without private source paths. Flush on orderly shutdown, retain earlier events on abnormal exit, report logger failures through stderr, and keep all file/image encoding off the UI thread. Record actual renderer provenance and orientation in snapshots. No continuous polling while ordinary viewing is idle.

Strengthen process tests for invalid initial input, repeated loading, stale generations, malformed/read-only inputs, close during loading, hangs, missing and blank evidence, and diagnostic write failures. Correlate frames/state/logs and source hashes. Measure packaged native M4 startup, large-image latency, repeated-load memory and idle CPU. Label renderer readback, native desktop observations, application-cold versus filesystem-cold measurements and unavailable GPU allocation counters honestly.

## Acceptance

- Headless checks and task graph validation pass.
- Native packaged M4 scenarios retain correlated logs/state/real frames and preserve source bytes.
- Failure-injection tests demonstrate rejected stale/blank/missing evidence, bounded timeouts and diagnostic failures.
- An initial reproducible performance report records hardware/build identity and limitations.
- Task/feature/user documentation reflects only verified behavior; deferred work is not marked completed.

## Unresolved decisions

M1 TASK-026–030 were subsequently adopted by the owner, who also supplied workflow priorities; see ../design/m1-decisions.md. Windows/Linux native desktop and license reviews remain deferred until requested.

## Completion

Local batch TASK-042–051, TASK-054/057/061 and product TASK-026–030 are complete. See [results](s0-hardening-results.md). The owner confirmed manual JPEG opening and adopted M1 recommendations. Fresh hosted CI remains outside this local completion claim; deferred Windows/Linux manual checks and license reviews are not marked complete.
