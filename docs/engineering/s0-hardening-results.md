# S0 hardening evidence — 2026-09-19

The requested local hardening batch and M1 decision tasks are complete. Native M4 checks combine automated evidence with the owner's manual JPEG-opening confirmation. **S0 is not yet closed:** the changed source/workflow has not been rerun on hosted Windows/Linux. Manual Windows/Linux desktop acceptance and manual license reviews are deferred by the owner.

## Implemented and checked

- Shared typed image errors; native path handling and explicit `--data-root` redirection. Configuration/cache locations are resolved without creating unused storage. Normal viewing survives unavailable diagnostic storage; explicit evidence failures return nonzero.
- Incremental JSONL on a bounded background channel, run/request/generation identity, backend and decode/upload/capture timings. Logging failures are reported; orderly exit flushes logs. An actual killed process retained startup records. Panic details omit potentially private payloads. Ordinary mode uses stderr unless an isolated data root is requested.
- Explicit renderer allocation readiness. The previous code could capture a blank large-image surface after decoding but before asynchronous texture upload. The application now retains the previous allocation until the new generation is allocated, rejects stale completions and allows only one active upload. Iced 0.14's existing runtime is referenced directly because its allocation re-export is gated on codec-enabled images; no additional decoder formats are enabled.
- Keyboard Tab focus and Return/Space activation of the sole Open button, alongside Cmd/Ctrl+O. Native close flushes logs without joining a decoder on the UI thread.
- Decode/EXIF/source-preservation tests plus malformed segment boundaries, read-only Unicode paths, dropping active work, failed replacement/cancel retention and stale upload/readiness checks.
- Eight native Metal process scenarios: empty, load, invalid initial input, failed replacement, repeated image, alternating landscape/portrait, 24 MP and 60 MP. Pixel checks verify actual bounds, aspect, placement and oriented color regions. Logs, frame states, generations and run identities must agree. Sources remain byte-identical.
- Adversarial tests reject blank/stale/missing evidence. A real hung child was killed/reaped at the deadline. Unwritable evidence initialization fails without damaging existing files; ordinary viewing still decoded when its requested log directory was unavailable.

The source checks are `cargo xtask check` (task/link graphs, formatting, strict Clippy, Rust tests). Fixture checks remain `cargo xtask fixtures`. Existing automated dependency policy remains in place; no manual license review was performed.

## Exact package and evidence

Current tested archive: `artifacts/hardening-package-5/lightwell-development.zip`.

Binary SHA-256: `6b92e4f7f998d702c955468c4c19bba4d405d29dc653acc9a6199af0c4782665`.

Lockfile SHA-256: `918f580c8443d810f7e5c34ff73829288dc5e1acf173c40eb5b2a03daeded67e`.

Ignored local evidence: `artifacts/hardening-ready-{empty,load,replacement,invalid,repeated,alternating,large24,large60}`, `artifacts/hardening-ready-process`, and `artifacts/hardening-ready-baseline`. Frame provenance is **window-renderer readback**, 1920×1280 at scale 2. Representative failed-replacement and 60 MP captures were visually inspected in addition to pixel checks. Earlier `hardening-baseline-*` runs before explicit allocation readiness contain blank images and are superseded; their timing numbers must not be used.

Native UI observations used the packaged application through computer-use accessibility and OS-window screenshots, separately from renderer capture. Verified: visible oriented photo, Cmd+O, Tab/Return opening the picker, Escape cancellation preserving the photo, native zoom-window resize with Fit, and close with exit 0 plus shutdown in isolated logs (`artifacts/native-session-2` and `native-session-3`). These observations preceded the final texture-readiness fix; final rendering is covered by packaged smoke. AX exposure remains limited.

**Native selection and automation limitation:** the native panel's Open button remained disabled for valid JPEGs selected by automation, including a temporary copied fixture. Removing the extension filter did not change the result; that experiment was reverted. Earlier Iced-probe selection succeeded, but it is not substituted for current packaged picker acceptance. The owner subsequently confirmed **“I can open a JPEG manually.”** This closes manual selection acceptance as owner-observed evidence; the automation itself did not complete selection. There was no approval-review rejection during these checks; the first restricted process launch aborted, and the authorized native launch route succeeded.

## M4 initial baseline

Apple M4 Pro, 14 CPU cores, 48 GiB unified memory, macOS 26.5.2 arm64, native Metal. Built-in Liquid Retina XDR: 3024×1964 panel, reported desktop 1800×1169 at 120 Hz; application captures use 2× scale. Local APFS SSD over Apple Fabric. Display profile/calibration and GPU allocation counters were not measured; this is S0 sRGB-renderer evidence, not professional monitor validation.

Thirty application-cold launches per workload, warm/uncontrolled OS file caches; no cache purge or filesystem-cold claim. The first run is retained separately in JSON. Launch-to-observed-frame is an upper bound sampled every approximately 50 ms (or process exit for fast runs). Request-to-capture includes event delivery, rendering and readback, not physical display scanout. Every loaded-image frame in this accepted baseline passes pixel checks.

| Metric | Empty | 24 MP | 60 MP |
| --- | ---: | ---: | ---: |
| Median launch to observed frame | 233 ms | 296 ms | 412 ms |
| p95 launch to observed frame | 241 ms | 353 ms | 470 ms |
| Median decode | — | 197 ms | 322 ms |
| Median allocation/upload completion | — | 10.7 ms | 11.5 ms |
| Median request to capture | — | 237 ms | 361 ms |
| p95 request to capture | — | 251 ms | 377 ms |
| Median sampled peak process RSS | 111 MiB | 506 MiB | 772 MiB |
| p95 sampled peak process RSS | 122 MiB | 516 MiB | 779 MiB |

Sixteen repeated 60 MP loads/captures in one process passed every pixel check and reached **965 MiB sampled peak RSS**. RSS rises across the capture sequence, with later increments near the 9.4 MiB readback size, so this short run is not evidence of a long-run memory plateau. Inspection confirms only one decode, one pending path, one upload and the previous/current image allocations are owned by application state; frame metadata contains no retained pixel arrays. Renderer/readback/allocator retention requires longer profiling before making a general no-growth claim. In ordinary loaded viewing after readiness and a one-second settle, a 30.16-second sample consumed **0.033% of one CPU core**; RSS was 757.13→757.34 MiB (peak 757.48). Ordinary idle has no polling or frame subscription. GPU/unified allocations are not separately added to RSS.

These are initial measurements, not accepted M1 performance budgets. Synthetic solid/quadrant JPEGs do not represent every photographic encoding or storage condition.

## Reproduction

Use fresh output paths and an unlocked native desktop:

```sh
cargo xtask check
cargo xtask fixtures
cargo xtask generate-fixtures
cargo xtask package --output artifacts/new-hardening-package
cargo xtask smoke --scenario large60 --output artifacts/new-large60
cargo xtask hardening --binary target/release/lightwell --output artifacts/new-failure-checks
cargo xtask measure --binary target/release/lightwell --output artifacts/new-baseline
```

The Rust measurement runner currently uses macOS `ps` accounting and is an M4 tool, not a portable performance claim. CI retains the existing three-platform builds/packages and now includes invalid/repeated/alternating/large-image Linux headless scenarios; those workflow changes have not yet executed remotely. Prior hosted results remain in [CI results](ci-results.md).
