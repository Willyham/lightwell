# Configured dependency review

Status: in progress, 2026-09-19. Project code is GPL-3.0-or-later. This is a local development build; redistribution readiness is not established.

`cargo-deny` 0.20.2 is pinned in the documented setup and verified by `cargo xtask audit`. `deny.toml` checks the configured macOS arm64, Windows x64 and Linux x64 graph, rejects unknown registries/git sources and requires an explicit license allowlist. License and source checks passed after reviewing BSL-1.0 in clipboard-win 5.4.1 and error-code 3.4.0. Boost is listed as GPL-compatible by the [GNU license list](https://www.gnu.org/licenses/license-list.html.en#boost). Their license was initially rejected by the allowlist, demonstrating the policy fails rather than silently accepting unknown licenses.

The raw cargo-deny check reports two maintenance findings, with no compatible removal through the selected released Iced stack:

- `paste` 1.0.15, through metal/wgpu: [RUSTSEC-2024-0436](https://rustsec.org/advisories/RUSTSEC-2024-0436).
- `ttf-parser` 0.25.1, through the Iced text stack and Linux decorations: [RUSTSEC-2026-0192](https://rustsec.org/advisories/RUSTSEC-2026-0192).

The owner authorized bounded review exceptions after upstream investigation. The maintained `cargo xtask audit` now passes under the [enforced advisory policy](advisory-policy.md): paste 1.0.15 expires on 2026-12-18, ttf-parser 0.25.1 on 2026-10-19 (exclusive, UTC). TASK-065 and TASK-066 track removal/re-review. Static deny.toml remains strict; a temporary config permits only these IDs after checking dates, exact versions and open tasks. New advisories still fail.

The ttf-parser upstream discussion includes a reported security issue without public details. Current exposure is installed system/bundled fonts; Lightwell offers no font import. The short exception covers local scaffold development, not distribution approval or a safety guarantee. Full asset/native notice review still keeps TASK-041 open.

`cargo xtask inventory --output DIR` records the host-resolved packages and copies top-level license/notice files. The M4 inventory contains 210 packages, including workspace crates. It is an inventory, not a complete notice audit: inspect embedded fonts/assets, native linking and license-file declarations, and assemble corresponding source before distribution. System macOS frameworks are runtime prerequisites, not bundled project code. Windows/Linux inventories must come from their configured native builds. No RAW library is enabled.

The package README explicitly carries these limitations. Dependency metadata currently includes local manifest paths for developer traceability; sanitize these before any external distribution.

Hosted confirmation: the dependency job in [run 35460353189](https://github.com/Willyham/lightwell/actions/runs/35460353189/job/105942978737) reproduced exactly these two advisory failures, with licenses and sources passing. This is now a blocking CI job; it is not configured with continue-on-error.

Local verification of the reviewed policy passed: six policy regression tests, six Rust core tests, formatting/lint/repository checks, and the full license/source/advisory command. A negative cargo-deny run withholding the ttf-parser exception failed as expected. Hosted verification passed in [run 35465688171](https://github.com/Willyham/lightwell/actions/runs/35465688171) at commit `c0ad340`: dependency audit, fixtures, all three build/package jobs and Linux headless smoke are green. The audit log records both exact versions, dates and follow-up tasks. The earlier failure above remains historical evidence.
