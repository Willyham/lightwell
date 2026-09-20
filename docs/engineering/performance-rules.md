# Performance rules for core and desktop changes

Status: **required engineering rules** for every change to `lightwell-core`, the desktop adapter or the JSON API. Budgets and measurement method live in [the performance specification](../specs/performance.md); recorded numbers live in [M1/M2 results](m1-m2-results.md) and [the preview optimization design](../design/preview-performance.md). This page exists so the same mistakes are not reintroduced while adding tools, modules, crop, export or RAW.

## Why these rules exist

The first M1 implementation was correct and fully tested, and every test ran on a 480×320 fixture. At 24 MP the same code re-read and hashed the original twice, decoded it again, cloned the frame, rewrote the whole frame once per transform layer, rendered a full frame to check one pixel and re-fetched the history page after every command. One pixel click cost about 330 ms and stalled every API client. None of that was visible in the test suite. The fixes were mechanical; the lesson is that correctness tests do not reveal per-operation cost, and that small fixtures hide it completely.

## Rules

1. **Decode the original once and reuse it by signature.** All request paths obtain pixels through `EditorService::verified_source`, which serves a cached decode when byte length, modification time, file identity and change marker are unchanged and otherwise decodes once and verifies the SHA-256 against the catalog. Never call `open_source` from a request path, and never hash the file on a cache hit.
2. **Share immutable pixels; never clone a frame you do not modify.** `SourceImage` and `Raster` hold `Arc<[u8]>`, the upload handle borrows the same allocation, and an identity recipe returns the source buffer itself. A new full-frame `Vec` needs a reason written in the code.
3. **Compile a recipe before rasterizing, and rasterize in one pass.** Exact geometry composes into one integer mapping; pixel replacements map through the suffix geometry after that pass. A new effect joins `compile` as an operation with declared input and output stages. Do not add a per-layer full-frame pass. An interpolating or color operation that cannot fuse must state its own stage boundary and be measured before merge.
4. **Point queries never rasterize.** Anything that answers "what is this pixel" or "what are these dimensions" uses `render::sample` or the compiled geometry and costs O(layers). This includes no-op detection, API sampling and future eyedropper or histogram-at-point features.
5. **The catalog owner thread does catalog work only.** It serves every client in turn, so a render there stalls the desktop and every agent. It may decode once on a cache miss; it never rasterizes, resizes, encodes or hashes pixels. Frame work goes to the preview worker, which keeps one active job plus one replaceable pending job and tags results with a generation, or to a new worker with the same bounds and cancellation.
6. **Every buffer and queue has a limit and a `ResourceLimit` error.** Current limits: 128 MiB encoded file, 64 megapixels, 16384 pixels per side, 512 MiB per evaluated frame, 100 history rows per page, 8 live clients, 256 buffered events, 1 MiB per request line. A new allocation that scales with image size or history length names the limit that bounds it. Silently exhausting memory is a bug; a clean limit error is acceptable.
7. **Desktop commands take the narrowest completion path.** A mutation calls the command, refreshes `asset.state`, requests one preview job and merges the resulting entry into the loaded page. A history selection requests only the preview job. A view change updates session state and re-renders nothing. Do not re-fetch the history page, re-render an unchanged raster or re-upload an unchanged allocation.
8. **Idle means asleep.** Blocking receives and blocking `accept` are correct; loops that wake to check a flag are not. The 16 ms preview poll subscription exists only while a preview is in flight and the 500 ms event sync only while an asset is open. A new timer must be gated the same way and must justify its interval against the idle CPU target.
9. **Parallelize above a measured threshold with the shared Rayon pool.** The transform pass goes parallel above one megapixel and stays serial below it. Do not create private thread pools or spawn a thread per row.
10. **History storage grows with stack size; do not add copies.** Each entry already stores its complete stack. Do not persist derived pixels, add per-entry buffers or materialize every snapshot to show the first page.
11. **Measure on photo-sized inputs before claiming anything.** Run the release diagnostic on a 24 MP and a 60 MP JPEG and record p50/p95, host, profile and cache state. The 480×320 fixture proves exactness, not cost. Timing gates do not belong in CI; exactness and bound tests do.

```sh
cargo xtask generate-fixtures --output fixtures/generated
cargo run --release --locked --package xtask -- editor-performance --source fixtures/generated/24mp.jpg --output artifacts/new-perf-24 --samples 30
```

## Review checklist

Answer each item in the commit or plan for any change under `crates/`:

- Which request paths read, hash or decode the original, and do all of them go through the cached verified source?
- Which new full-frame allocations exist, which limit bounds each one, and which are shared rather than cloned?
- Does any point query, validation or no-op check render a frame?
- What now runs on the owner thread, and is any of it frame work?
- Which desktop messages trigger `asset.state`, `history.list`, a preview job or an upload, and is each one necessary for that message?
- Which timers, polls or subscriptions were added, and what gates them?
- What did `editor-performance` report on 24 MP before and after, and where is that recorded with its scope?
- Which tests prove exactness against a stepwise reference for new effects, and which prove buffer sharing where sharing is claimed?

## Known remaining costs

These are accepted or pending decisions. Do not "fix" them without the referenced scope.

| Cost | Status | Constraint on a fix |
| --- | --- | --- |
| Full-resolution GPU upload for every preview, including Fit | Open; largest per-interaction cost at 60 MP | A bounded Fit preview must keep 100% inspection exact and source-detail-ready; design it against [the editor specification](../specs/single-image.md) first |
| One decode on the owner thread per catalog reopen | Accepted | Moving it needs the cache shared with the preview worker; not worth it before M3 |
| 500 ms event poll while an asset is open | Accepted until push notifications exist | Do not shorten the interval |
| `snapshots` and `snapshot_layers` tables are written but never read | Pending owner decision | Changing them changes the catalog format and needs an explicit tested conversion |

## Anti-patterns already removed

| Seen in this repository | Replacement |
| --- | --- |
| Read, hash and decode the original on every preview, sample and edit | Signature-validated decoded-source cache |
| Clone the source into the render buffer, then allocate a new frame per transform layer | Composed geometry, one parallel pass, shared identity buffer |
| Render a full frame to compare one pixel for no-op detection | `render::sample` |
| Re-fetch state and the 50-row history page after every command, and re-render on zoom | Narrow command, preview and session tasks with entry merge |
| Accept loop sleeping 10 ms in a non-blocking spin | Blocking `accept` woken by one loopback connection on shutdown |
| Tiny fixture as the only timing evidence | `editor-performance` runner on generated 24 MP and 60 MP inputs |
