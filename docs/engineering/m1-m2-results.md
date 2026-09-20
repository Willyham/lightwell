# M1/M2 implementation and verification

Status: **implemented and locally verified on 2026-09-20**. M1 provides the persistent history foundation and M2 adds exact quarter-turn and reflection operations. M3 modules, M4 crop, export, Locate and MCP remain planned work.

## Tested build

The native journey used the unsigned arm64 macOS package produced by `cargo xtask package` from application revision `64f9d1c520c32ea9da939970e7eeb187446cef55` on the owner's M4 MacBook Pro.

| Identity | Value |
| --- | --- |
| Target/profile | `aarch64-apple-darwin` / release |
| Application binary SHA-256 | `6a88a79dd944dd2a1b5107d2e7ddbffc7f3ab16cf78e442b168c8e2726a4f698` |
| `Cargo.lock` SHA-256 | `762f195582c5d19c6c928908e97739adc7e3c196105566823a92a6d8a9ca0a56` |
| Package SHA-256 | `0fceb7d1a7d2037feb7b8020585f1a0574c4d757568c5a55438e6e35d3f00c41` |
| Fixture | `fixtures/s0/orientation-1.jpg` |
| Fixture SHA-256 | `004676dfc3fcf39f11ea41a6997104192e941cc4a89ec34a396907e77b166d8b` |

The package manifest reported a clean source tree. Later documentation and acceptance-runner changes do not change the tested application binary.

## Native M4 journey

An OS-window screenshot inspection exercised the packaged editor at 1280×800 logical pixels with an isolated catalog. The rendered fixture, status text, history rows and current entry/snapshot/source identities were visually correlated at each step.

1. Import created Original, then two same-coordinate pixel edits created sequences 1 and 2 and two ordered layers.
2. The 100% view reported a 2.00× display scale and mapped one source pixel to one physical framebuffer pixel.
3. Selecting Original showed `Previewing history`, disabled edit controls and exposed Return/Restore without moving the committed pointer.
4. Restore Original appended sequence 3 and produced an empty current layer stack.
5. Rotate right, Mirror horizontal and Flip vertical appended sequences 4–6. The asymmetric fixture changed between 480×320 and 320×480 as expected, with no interpolation.
6. Two undos and one redo selected the correct retained entries. Cmd+Z and Shift+Cmd+Z were also verified against visibly different transformed content.
7. While the GUI still previewed Original, an authenticated loopback client called `edit.transform` with actor `native-api-client`. The API returned revision 10 and entry `entry-f80182ebc0c548b8bdb455bcfac8a81d`; the GUI added attributed sequence 7 while preserving the historical preview. Return to current displayed the external result.
8. Direct competing SQLite access failed with `database is locked`, demonstrating the single-owner boundary. Closing and reopening the same package/catalog restored all eight entries, current pixels, identities and navigation state.

The live API's `schema.list` response exposed import, state, paginated history, inspect, pixel and transform edits, undo/redo/restore, preview selection, viewport state, pixel sampling and event replay. The live-session file was created with a loopback address and per-run token and removed on orderly shutdown.

After 2 minutes 16 seconds settled with the photograph loaded, the process reported 0.2% instantaneous CPU, 1.83 seconds accumulated CPU time and 103,344 KiB RSS (about 100.9 MiB). These are observations for this build and fixture, not accepted budgets. The native run proves macOS behavior; it does not transfer GPU or performance claims to Windows or Linux.

## Automated acceptance

`cargo xtask check` passed repository/task validation, formatting, Clippy and all tests: 9 desktop unit tests, 1 JSON subprocess integration test, 28 core tests and 13 xtask tests, with one existing ignored xtask test. Core coverage includes every EXIF orientation, exact buffers, no-op and invalid edits, revisions/request deduplication, atomic rollback, competing owners, restore ancestry, historical preview during an external change, event gaps/reconnect, bounded preview scheduling and subprocess EOF.

The release-profile command below created an ignored evidence bundle and passed an exact 208-entry journey:

```sh
cargo run --release --locked --package xtask -- editor-acceptance \
  --output artifacts/m1-m2-release-20260920
```

| Release measurement | Result |
| --- | ---: |
| Import | 2.660 ms |
| Historical preview | 0.513 ms |
| 200 sequential exact transform commits | 163.016 ms |
| Page 208 entries, 25 per query | 4.598 ms |
| Catalog reopen | 0.208 ms |
| Render current 203-layer recipe | 25.003 ms |
| Complete journey | 201.674 ms |
| Catalog after 208 entries | 13,119,488 bytes |

The run finished at revision 209 with a 320×480 image, stable original/A identities and the original source hash unchanged. Timings use a warm filesystem cache on this host and are diagnostic observations, not cross-platform budgets. Use a new output directory for every rerun.

## Preview latency optimization

The later preview optimization working tree based on `c01f4c83ac3636a5c3a660ebcaacbb2bab2c6939` keeps one signature-validated decoded source, shares immutable source/render/upload bytes, composes exact transform stacks into one pass, parallelizes large transforms and avoids full state/history reloads for selection-only UI actions. Mutations now refresh current state and merge its entry into the bounded UI page instead of querying the complete page again. Zoom changes no longer decode or rerender unchanged pixels. Source replacement checks include byte length, modification/change timestamps and file identity; a cache miss still verifies the decoded SHA-256 against the catalog.

A solo release acceptance rerun reduced the 203-layer render from a same-session pre-change baseline of 25.364 ms to 0.244 ms. Historical preview in that small exact journey measured 0.035 ms. A separate generated 24 MP (6000×4000, SHA-256 `b54c2a158a3d384674f5d731f940d553039d61f51b83a0e7b1e3b0247aa056eb`) release diagnostic recorded 30 samples:

| Core measurement | Result |
| --- | ---: |
| Import | 57.652 ms |
| Cached preview-job lookup | 0.086 ms |
| Original render using shared pixels | 0.0003 ms |
| One transform | p50 11.650 ms; p95 13.641 ms |
| 200 composed transforms | p50 13.375 ms; p95 14.375 ms |
| Reopen, decode and construct preview job | 30.169 ms |

The generated 60 MP workload (10000×6000, SHA-256 `b9e0118ab69b5d889b62087759be0b33f41b010f8d5bf14d2864dd9e47340221`) also used 30 samples. One transform measured 24.615 ms p50 / 27.778 ms p95; 200 composed transforms measured 27.106 ms p50 / 29.689 ms p95. Import measured 125.793 ms and reopen/decode/job construction measured 74.417 ms.

`cargo xtask check` passed with 31 core tests, including exhaustive three-transform/pixel interleavings and source-cache invalidation. These measurements cover core request-to-render work on the native M4 with a warm filesystem cache; they exclude desktop task scheduling, GPU upload and presentation. A fresh packaged GUI journey and end-to-end input-to-present capture have not been run for this optimization, and Windows/Linux performance remains unverified.

### Pixel sampling follow-up

After the optimization above, the pixel no-op check and `render.sample` evaluate one output pixel by compiling the recipe and mapping the coordinate, without rasterizing. On the same generated 24 MP release diagnostic (warm cache, core only), a pixel edit after one rotate measured 0.2 ms instead of 17.8 ms and ten sequential pixel edits 3.1 ms instead of 131.7 ms; a full two-pixel-plus-rotate render remained about 17 ms. `cargo xtask check` passes with 33 core tests, including a sample-versus-render comparison over every pixel of an interleaved recipe. The desktop now reports a catalog owned by another instance as a startup error instead of a panic, the loopback listener blocks in `accept` while idle, and `catalog.list` exposes referenced assets. These are diagnostic observations on the native M4; no packaged GUI journey was rerun. Catalogs written by this build are format 1; the later versions work converts them to format 2 on open, as recorded in [versions and lineage](../design/versions-and-lineage.md).

## Exact geometry contract

Coordinates use a top-left origin, x right and y down. Every layer addresses its input stage after EXIF orientation is applied once.

| Operation | Input-to-output mapping | Output size |
| --- | --- | --- |
| Rotate right | `(x, y) → (h - 1 - y, x)` | `h × w` |
| Rotate left | `(x, y) → (y, w - 1 - x)` | `h × w` |
| Mirror horizontal | `(x, y) → (w - 1 - x, y)` | `w × h` |
| Flip vertical | `(x, y) → (x, h - 1 - y)` | `w × h` |

These mappings are integer-exact. Four matching quarter-turns and two matching reflections are identities. Operation order remains observable: a pixel edit before a transform moves with the image, while one after a transform uses the transformed dimensions.

## Remaining limits

- Native Windows/Linux desktop behavior and performance remain deferred; automated portable compilation/tests are not native GPU evidence.
- The custom Iced controls have visible labels and verified keyboard shortcuts, but native screen-reader exposure is not yet verified.
- Rendering still uses full decoded RGBA buffers, but unchanged sources are cached once, immutable pixels are shared and an exact transform stack evaluates in at most one full-image pass. Inputs are bounded to 128 MiB JPEG files and 64 megapixels; render estimates are capped at 512 MiB, and preview work is one active plus one replaceable pending job.
- Catalogs are internal v0 data with an explicit format marker. Incompatible data, changed/missing sources and unknown operations fail explicitly instead of being discarded. A catalog is not a backup of the original.
- Export, manual Locate, MCP, crop, a generic module host, RAW and broader color-management work are not implemented. Manual license/native/asset review also remains deferred.
