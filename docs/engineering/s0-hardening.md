# S0 hardening contract

Status: implemented and verified locally; S0 is accepted. Exact package identities, native evidence, measurements and limitations are in [hardening results](s0-hardening-results.md). Fresh hosted verification and deferred native Windows/Linux/license review remain unfinished.

## Implemented behavior

- Typed image errors at the shared service boundary; source bytes are read-only.
- Native configuration/cache/log path resolution without creating unused state. An isolated data root redirects diagnostics; ordinary viewing remains usable if diagnostics cannot be written.
- Fresh, bounded opt-in evidence directories with explicit write failures.
- Incremental JSONL diagnostics on a bounded background channel, with run/build/request/generation identities and timings but no private source paths. Orderly shutdown flushes; abnormal exit retains written events.
- File/image encoding off the UI thread, explicit renderer-allocation readiness and stale-generation rejection.
- One active and one latest pending decode, bounded uploads/previews, and no polling during ordinary idle.
- Process checks for invalid, repeated and alternating requests, malformed/read-only inputs, close during load, hangs, missing/blank evidence and diagnostic failures.

## Verification contract

Correlate actual rendered frames with state, logs, build identity and unchanged source hashes. Reject stale, blank or missing evidence. Enforce bounded timeouts and child cleanup. Measure packaged native startup, large-image latency, repeated-load memory and idle CPU with hardware/build identity.

Distinguish renderer readback from OS-window interaction, application-cold from filesystem-cold measurements, and process RSS from unavailable GPU allocation counters. Owner-observed manual JPEG opening is separate from automated picker-selection evidence. Headless Linux functional results are not native desktop/GPU measurements.

Keep feature and user documentation aligned with verified behavior. The [platform matrix](platforms.md) and [CI results](ci-results.md) describe outstanding checks.
