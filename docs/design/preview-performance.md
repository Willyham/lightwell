# Preview latency optimization

Status: **implemented and locally verified on 2026-09-20**. This work optimizes the implemented M1/M2 editor without changing recipe, history, source-integrity or exact-transform semantics. The provisional interaction budgets remain defined in [the performance specification](../specs/performance.md); the rules this work established for later changes are in [performance rules](../engineering/performance-rules.md).

## Problem

The initial history implementation deliberately favored a simple correctness proof. Each displayed state currently:

1. reads and hashes the complete JPEG;
2. reads and decodes the JPEG again;
3. clones the decoded RGBA source into a render buffer; and
4. allocates and rewrites a complete RGBA buffer for every transform layer in the selected recipe.

The desktop adapter also reloads state and the first history page for selection-only commands, and it starts a new render for view-only zoom changes. These costs make transforms and history browsing visibly lag behind the persisted mutation itself.

## Design

- Keep one decoded source cache in the single catalog owner. A cache hit requires unchanged byte length, modification time and platform file identity. A changed signature triggers a fresh bounded decode and SHA-256 comparison before pixels are reused. Cache entries are immutable, shared pixel storage and remain bounded to the currently used asset.
- Use shared immutable byte storage between the source, render result and Iced upload handle. An unedited/orientation-equivalent result can reuse source pixels without another full-buffer copy.
- Compile a recipe before rasterization. Compose all exact quarter-turn/reflection layers into one integer affine mapping, transform the source in at most one full-image pass, then map ordered pixel replacements into final coordinates. The output must remain byte-exact for every existing operation order.
- Keep preview scheduling at one active plus one replaceable pending job. Do not introduce an unbounded snapshot or texture cache.
- Give selection-only and view-only UI commands narrow completion paths. History selection fetches only the requested immutable entry/source job; zoom and pan update session state without decoding or rerendering unchanged pixels.
- After an edit/undo/redo/restore, refresh only current state and merge its immutable entry into the bounded 50-row UI page instead of querying and deserializing that page again.
- Parallelize the single transform pass above one megapixel with the pinned Rayon worker pool. Small images stay serial to avoid scheduling overhead.
- Evaluate single pixels without rasterizing. The pixel no-op check and `render.sample` compile the recipe, map the coordinate through the composed geometry and take the last replacement that lands on it, so their cost is linear in the layer count rather than the image area. The catalog owner thread therefore never rasterizes for API calls; it only decodes once on a cache miss.
- Keep the loopback listener blocked in `accept` while idle instead of polling every 10 ms; shutdown wakes it with one loopback connection.

## Constraints

- Original files remain read-only. Changed, replaced, missing and invalid sources must still fail explicitly.
- Recipe snapshots and history entries remain complete and immutable; no derived pixels are persisted in the catalog.
- Rendering stays off the UI thread, stale generations remain suppressed, and the source/render allocation bounds remain enforced.
- Transform output and pixel-operation order must be identical to the accepted M2 coordinate contract on every platform.
- Timing evidence uses optimized builds. Shared-runner timing is diagnostic rather than a CI gate.

## Acceptance

1. Existing exact-buffer, history, recovery, API and queue tests pass unchanged.
2. New tests cover transform composition, interleaved pixel/transform order, shared original pixels, source-cache invalidation and narrow UI command behavior where practical.
3. `cargo xtask check` passes.
4. The release editor acceptance journey reports a material reduction from the recorded local baseline of 25.364 ms for its 203-layer render. A generated 24 MP diagnostic records cold import, cached history preview and transform render latency separately when the fixture is available.
5. Documentation reports measurements with fixture, profile and host scope, without presenting local diagnostic observations as cross-platform guarantees.

## Verification result

`cargo xtask check` passes with 31 core tests, including all 64 three-transform combinations interleaved with pixel replacements, shared source/render allocation checks and same-length source replacement invalidation. Ten desktop tests, the JSON subprocess test and 13 ordinary xtask tests also pass; the existing sleeping timeout helper remains intentionally ignored.

On the owner's arm64 M4 MacBook Pro, a solo release acceptance run reduced the 203-layer 480×320 render from the turn baseline of 25.364 ms to 0.244 ms. The generated 6000×4000 JPEG diagnostic used 30 samples: one transform measured 11.650 ms p50 / 13.641 ms p95, while 200 composed transforms measured 13.375 ms p50 / 14.375 ms p95. The cached preview-job lookup was 0.086 ms. Import was 57.652 ms; reopening the catalog and reconstructing the decoded source/job was 30.169 ms. A separate 30-sample 10000×6000 run measured one transform at 24.615 ms p50 / 27.778 ms p95 and 200 transforms at 27.106 ms p50 / 29.689 ms p95.

A follow-up on the same host replaced the full-frame render inside the pixel no-op check with the sampling path. On the same generated 24 MP JPEG in release, a pixel edit after one rotate fell from 17.8 ms to 0.2 ms and ten sequential pixel edits from 132 ms to 3 ms, while a full render of the same two-pixel-plus-rotate recipe stayed at about 17 ms. The new render test compares every sampled pixel against the rasterized output for an interleaved pixel/transform recipe, including source alpha.

These are core request-to-render measurements with a warm filesystem cache. They exclude task scheduling, GPU upload and presentation, so they do not establish the end-to-end input-to-present budget or Windows/Linux performance. The exact command is documented in [scaffold commands](../engineering/scaffold-commands.md).
