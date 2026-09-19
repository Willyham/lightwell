# CI activation and additional checks

Scope: push the reviewed scaffold, inspect its first GitHub Actions run, fix actual build failures, and automate existing fixture/dependency checks. This does not close native desktop acceptance or add editor functionality.

Keep one shared runner for local/CI checks. Add a fixture operation invoking the existing pinned Pillow corpus checker. Run it in an Ubuntu quality job with an explicit Python environment. Add a separate blocking dependency job using pinned cargo-deny 0.20.2; preserve actionable output. Known maintenance advisories require individually documented review, never blanket suppression. New vulnerability/license/source findings must fail.

The existing native build matrix remains macOS, Windows and Ubuntu with locked debug checks, optimized build, package and seven-day artifacts. Inspect failures by exact run/commit identity. GUI smoke may run on Linux Xvfb with a software Vulkan driver if it works, but must be labeled headless/software and must not count as accepted native GPU/desktop evidence.

Acceptance: real GitHub run results for the pushed commit, fixture validation exercised on a hosted runner, auditable dependency results, and clear reports for anything unavailable. Retain failure logs and update the task/feature/command documentation. Avoid claiming that configured steps passed until their run completes.
