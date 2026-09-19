# Configured dependency review

Status: in progress, 2026-09-19. Project code is GPL-3.0-or-later. This is a local development build; redistribution readiness is not established.

`cargo-deny` 0.20.2 is pinned in the documented setup and verified by `cargo xtask audit`. `deny.toml` checks the configured macOS arm64, Windows x64 and Linux x64 graph, rejects unknown registries/git sources and requires an explicit license allowlist. License and source checks passed after reviewing BSL-1.0 in clipboard-win 5.4.1 and error-code 3.4.0. Boost is listed as GPL-compatible by the [GNU license list](https://www.gnu.org/licenses/license-list.html.en#boost). Their license was initially rejected by the allowlist, demonstrating the policy fails rather than silently accepting unknown licenses.

The advisory check fails with two unresolved maintenance findings, with no safe upgrade reported:

- `paste` 1.0.15, through metal/wgpu: [RUSTSEC-2024-0436](https://rustsec.org/advisories/RUSTSEC-2024-0436).
- `ttf-parser` 0.25.1, through the Iced text stack and Linux decorations: [RUSTSEC-2026-0192](https://rustsec.org/advisories/RUSTSEC-2026-0192).

No ignores or broad exceptions were introduced. Review upstream migration or a narrowly justified temporary maintenance exception before completing TASK-041. These are maintenance advisories, not a claim of an identified exploitable vulnerability in this application.

`cargo xtask inventory --output DIR` records the host-resolved packages and copies top-level license/notice files. The M4 inventory contains 210 packages, including workspace crates. It is an inventory, not a complete notice audit: inspect embedded fonts/assets, native linking and license-file declarations, and assemble corresponding source before distribution. System macOS frameworks are runtime prerequisites, not bundled project code. Windows/Linux inventories must come from their configured native builds. No RAW library is enabled.

The package README explicitly carries these limitations. Dependency metadata currently includes local manifest paths for developer traceability; sanitize these before any external distribution.
