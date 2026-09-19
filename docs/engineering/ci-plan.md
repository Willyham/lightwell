# CI activation and additional checks

Scope: push the reviewed scaffold, inspect its first GitHub Actions run, fix actual build failures, and automate existing fixture/dependency checks. This does not close native desktop acceptance or add editor functionality.

Keep one shared runner for local/CI checks. Add a fixture operation invoking the existing pinned Pillow corpus checker. Run it in an Ubuntu quality job with an explicit Python environment. Add a separate blocking dependency job using pinned cargo-deny 0.20.2; preserve actionable output. Known maintenance advisories require individually documented review, never blanket suppression. New vulnerability/license/source findings must fail.

The existing native build matrix remains macOS, Windows and Ubuntu with locked debug checks, optimized build, package and seven-day artifacts. Inspect failures by exact run/commit identity. GUI smoke may run on Linux Xvfb with a software Vulkan driver if it works, but must be labeled headless/software and must not count as accepted native GPU/desktop evidence.

Acceptance: real GitHub run results for the pushed commit, fixture validation exercised on a hosted runner, auditable dependency results, and clear reports for anything unavailable. Retain failure logs and update the task/feature/command documentation. Avoid claiming that configured steps passed until their run completes.

## Current scaffold closure pass

Continue TASK-052/053/055/056/062/063 with the hardened viewer and optimized developer launch. Preserve the existing automatic dependency policy while manual license and Windows/Linux desktop checks remain deferred. Add package build identity (target, binary/lock hashes and source revision/dirty flag), record hosted runtime imports, and exercise the actual packaged Linux executable in software-rendered CI. Verify a fresh packaged Mac binary against the existing synthetic capture checks. Publish only tracked code, documentation and synthetic fixtures; the owner's JPEG and local captures stay ignored.

The bootstrap playbook must provide fresh-output setup, launch, smoke, failure inspection and packaging steps without reconstructing historical reports. A current hosted run and its retained package/smoke artifacts are required before closing the S0 gate. Only then may TASK-064 finalize the already adopted M1 decisions. Missing or failed evidence remains outstanding, not inferred from an older green run.
