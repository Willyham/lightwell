# Rust-only development tooling

Status: implemented and verified locally (TASK-069); fresh hosted verification is tracked separately by TASK-052/053/055/056. Owner decision: use Rust for all project tooling; a separate Go API test client may be added later if useful. No Go prerequisite is introduced.

Implement development tooling in the Rust `xtask` crate. Retain `cargo xtask` commands for Doctor, optimized develop (explicit --debug), locked checks/builds, package/inventory/audit, fixtures and smoke. Add native Rust entry points for capture inspection, large-fixture generation, process hardening and macOS measurement. Keep output freshness, deadlines, child cleanup, hashes, run/generation/backend checks, pixel tolerances, source preservation and advisory expiry checks. Shells are not used to interpolate paths. Errors exit nonzero and evidence remains inspectable.

Use pinned tooling dependencies, isolated from app runtime. Do not introduce app feature work, new manual license checks or manual Windows/Linux gates. Rust tests cover runner regressions. CI and contributor instructions use only Rust and documented platform tools.

The sixteen checked-in JPEGs and manifest are golden inputs, checked by hashes, decoded metadata and independent color/orientation expectations. Generated 24/60 MP workloads use a deterministic Rust generator with quadrant colors and direction/detail marks. Verify deterministic generation and avoid source overwrites. ICC/CMYK goldens stay in the checked-in corpus; reusing the decoder is not an independent color oracle.

Acceptance: all maintained commands execute through Rust, both task graphs and documentation links validate, source and golden hashes stay unchanged, process/pixel/policy regressions pass, a native packaged replacement/large-image capture and failure suite pass, and hosted CI uses the new runner. Record platform limits and current hosted status separately. Keep all durable application/API logic in Rust; later Go tests must use the external protocol rather than bypass it.


## Maintained commands

| Purpose | Command |
| --- | --- |
| Environment, full checks, repository checks | `cargo xtask doctor`, `cargo xtask check`, `cargo xtask check-repository` |
| Format/lint/test/build | `cargo xtask fmt`, `cargo xtask lint`, `cargo xtask test`, `cargo xtask build --release` |
| Normal / debugger launch | `cargo xtask develop`, `cargo xtask develop --debug` |
| Golden corpus / new large workloads | `cargo xtask fixtures`, `cargo xtask generate-fixtures --output NEW_DIRECTORY` |
| Package / notices inventory | `cargo xtask package --output NEW_DIRECTORY`, `cargo xtask inventory --output NEW_DIRECTORY` |
| Existing automatic policy | `cargo xtask audit` |
| Native smoke | `cargo xtask smoke --binary PATH --scenario replacement --output NEW_DIRECTORY` |
| Existing screenshot inspection | `cargo xtask check-capture --image PATH --orientation 6` |
| Native process failure checks | `cargo xtask hardening --binary PATH --output NEW_DIRECTORY` |
| macOS measurement | `cargo xtask measure --binary PATH --output NEW_DIRECTORY --samples 30` |
| Historical trial binaries | `cargo xtask probe --candidate iced --output NEW_DIRECTORY` (or `egui`) |

Commands reject unknown arguments; paths are passed directly to child processes without shell interpretation. `build` remains debug unless `--release` is specified. Ordinary `develop` remains optimized. `measure` explicitly rejects non-macOS hosts because its accounting uses macOS `ps`; no Windows/Linux performance claim is made. Native prerequisites still include SDKs, drivers, Linux display libraries/XDG portal, and an available graphical session. First compilation downloads/builds pinned crates using the Rust toolchain and lockfile.

The Rust task checker implements the checked-in schema vocabulary and external gates, rather than depending on a user’s installed skill.

## Local verification

`cargo xtask check`, `doctor`, `fixtures`, `package`, and the existing automated `audit` all pass. Rust tests cover schema rejection, golden images, advisory expiry/version/source/task failures, existing-output preservation, ZIP/tar payloads and ZIP executable mode, blank/stale/missing evidence, and actual child timeout cleanup. The one ignored test is a deliberately sleeping helper launched explicitly by the timeout test, not a skipped acceptance check. App/core tests also pass (26 tests total, plus the helper).

`artifacts/rust-tooling-package` contains the tested Mac package. Its binary SHA-256 is `505bfd3cdf7a848583420afee1e1fa84769116d6e53da05a09d774df62fccd74`, unchanged by the tooling migration. The new workspace lock hash is `2ed74a57bb720385de9537a6ac5c364939b556470113b52152a29f805fd0017b`; tooling dependencies do not become app runtime dependencies.

All eight scenarios pass in `artifacts/rust-smoke-{empty,load,replacement,invalid,repeated,alternating,large24,large60}` on native M4 Metal. The 60 MP renderer frame was visually inspected: correct quadrant colors, complete Fit image, no clipping and correct dimension label. `artifacts/rust-hardening/result.json` passes actual timeout/reaping, diagnostic initialization failure, continued ordinary decode without writable diagnostics and abrupt-exit log preservation.

`artifacts/rust-measure/measurements.json` validates the new measurement runner with one launch per empty/24/60 MP workload, sixteen repeated 60 MP opens and a thirty-second idle observation. This is runner validation, not a replacement statistical baseline. Existing 30-run reports remain tied to their original inputs.

The new 24 MP hash is `b54c2a158a3d384674f5d731f940d553039d61f51b83a0e7b1e3b0247aa056eb`; 60 MP is `b9e0118ab69b5d889b62087759be0b33f41b010f8d5bf14d2864dd9e47340221`. Release generation in `artifacts/rust-generated-fixtures` and debug regeneration in `artifacts/rust-generated-repeat` were byte-identical, including the manifest. All sixteen tracked golden inputs and the private owner JPEG remain unchanged.

Documentation describes the current Rust tooling and fixture contracts; superseded setup and generator provenance are omitted.
