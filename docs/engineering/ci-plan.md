# CI configuration and outstanding verification

The checked-in workflow uses the same Rust xtask commands as local development. It runs locked checks, optimized builds and development packaging on macOS, Windows and Ubuntu, with seven-day artifact retention. Separate jobs validate deterministic fixtures and the pinned cargo-deny dependency policy.

Package evidence records target, binary and lockfile hashes, source revision and dirty state. Runtime-import collection is configured. Linux smoke drives the packaged executable under Xvfb/software Vulkan, including invalid, repeated, alternating and large-image scenarios. This is functional headless evidence, not native desktop or GPU-performance acceptance.

## Verification requirements

- Inspect actual run results for the tested commit and artifact identities; configured steps are not passing results.
- Preserve blocking license/source/advisory checks and narrowly scoped, expiring maintenance exceptions. New or unapproved findings fail.
- Inspect Windows/Linux runtime imports and retained package/smoke artifacts.
- Publish only project code, documentation and synthetic fixtures; private originals and local captures remain excluded.
- Use fresh output directories and retain correlated state, logs, frames, source hashes and reproduction commands.
- Distinguish native M4 checks, headless Linux checks, hosted compilation and deferred native Windows/Linux sessions.

## Current status

S0 is accepted on native M4 evidence and the verified portable run identified in [CI results](ci-results.md). The closure snapshot's expanded workflow and current Windows/Linux packages still need fresh hosted execution and artifact inspection. Source push requires authorization; this plan does not authorize publishing.

The [bootstrap playbook](bootstrap-playbook.md) is locally verified using a clean checkout with a warm shared Cargo cache. Its current Windows/Linux package verification remains unfinished. Manual Windows/Linux desktop and manual license reviews are deferred. M1/M2 are locally verified on macOS; fresh hosted checks for their expanded dependency graph and editor binaries remain to run.
