# Performance and correctness measurement plan

Status: **provisional targets, not measured results**. The owner's M4 MacBook Pro is the first reference target. Exact RAM/chip configuration, minimum hardware, catalog scale and supported OS versions remain to be recorded. Fast interaction, background throughput, and output correctness must be evaluated separately.

## S0 first-build baseline

The first build is now the [image-loading skeleton](bootstrap.md). Measure optimized-build startup, request-to-present JPEG loading, idle redraw/CPU and repeated-open memory on M4 through the structured logging/smoke workflow. No geometry, export, catalog or RAW benchmark gates S0. Record 24/60 MP behavior and resource limits; the table below remains a provisional M1 target set. S0 must still keep allocations/queues bounded and loading responsive. TASK-061 collects evidence; TASK-063 requires native M4 acceptance and automated portable checks; Windows/Linux manual checks are deferred.

## Proposed reference workloads

| Workload | What it reveals |
| --- | --- |
| 24 MP and 60 MP RGB JPEGs, rotated EXIF variants, embedded sRGB/Adobe RGB/Display P3 profiles | First-open latency, memory, geometry and color |
| Huge/invalid dimensions, truncated files, malformed profiles | Resource bounds and error recovery |
| M4 MacBook Pro with its actual display resolution/scaling recorded; optional external SDR 4K case | Primary preview/input, color, text/DPI and Metal resource measurements |
| Linux ARM64 VM on the Mac; later native Windows/Linux GPU machines | Functional portability versus native GPU behavior, measured separately |
| 100,000 metadata rows; 1,000,000-row stress catalog with skewed dates/ratings | Index selection, pagination, filtering, startup independent of total image bytes |
| At least 1,000 real images for warm/cold browsing, then a larger owner dataset | Thumbnail decode and cache behavior that synthetic rows cannot establish |
| Local SSD; later removable SSD and representative NAS source storage | Distinguish CPU/GPU throughput from storage latency |
| Nikon Z6 NEF and Fujifilm X100VI RAF, including the owner's actual compression/bit-depth modes | Later RAW decode, image quality and peak memory; sample dimensions must be recorded rather than inferred from brand |

All datasets need provenance, dimensions, profile/orientation and redistribution permission. Use synthetic geometry/color fixtures in the repository and a documented local manifest for private camera originals. Never commit an owner's photo library by default.

## Provisional budgets

Apply the initial budgets to the owner's M4 MacBook Pro, recording unified-memory capacity, exact chip configuration, storage, macOS and display settings before running measurements. Its RAM capacity has not been established; 16 GB is no longer an assumed machine specification or minimum requirement. Keep the existing budgets as engineering hypotheses until measured and accepted; do not claim they were agreed or achieved. Measure first in SDR with the display color contract recorded.

| Metric | Proposed budget | When |
| --- | --- | --- |
| Launch to usable empty shell | p95 < 1 s warm; < 2 s cold | M1 |
| Select uncached local 24 MP JPEG to fit preview | p95 < 750 ms; show loading feedback within 100 ms | M1 |
| Crop overlay frame time with loaded preview | p95 <= 16.7 ms at 60 Hz | M1 |
| Geometry input to presented preview | p95 < 50 ms after source preview is ready | M1 |
| Empty steady-state process memory | Target <= 150 MiB, measured per OS including helper processes | M1 |
| 24 MP one-image edit working set | Target <= 600 MiB CPU-resident allocation/RSS accounting reported separately | M1 |
| 60 MP import/export peak | Target <= 1 GiB total process RSS; GPU memory reported independently and unified-memory overlap explained | M1 |
| Idle app CPU | Target < 1% of one core over 30 s after background work settles | M1 |
| First page of 100,000-row indexed filter | p95 < 100 ms; warm metadata, thumbnails excluded | M2 |
| Warm adjacent-image fit preview | p95 < 150 ms, cache hit | M2 |

Do not establish an export throughput claim before measuring JPEG decode, transform/color and encoding separately. Report wall time, megapixels/sec, peak memory, and cancellation latency. No full float32 60 MP RGBA allocation is needed for the crop prototype: that single buffer alone would be about 916 MiB, before decoded bytes and GPU copies.

## Method and reporting

Use optimized builds with recorded commit, dependency lockfile, OS, CPU/GPU/driver, RAM, display resolution/profile, and storage type. Report cold and warm runs separately, define whether “cold” means application caches or OS filesystem caches, and exclude neither failures nor long-tail samples without explanation. Collect at least 30 latency samples for initial comparisons; use longer runs when investigating tails. Do not compare debug builds with release builds.

Instrument time from user event to presented frame rather than reporting shader duration as interaction latency. Include decode, color transform, upload, queue delay and presentation. Measure CPU RSS, cache bytes, GPU allocations, and transient copies; avoid adding shared CPU/GPU allocations twice on unified-memory systems. Capture idle behavior after all background work stops.

Use deterministic geometry fixtures, EXIF orientations 1–8, gradients, color patches, fine detail, and photo references. Check coordinate round trips, crop coverage, CPU/GPU render differences and export interpretation. Verify centered proportional Option resizing and composition-preserving straightening at multiple zoom/DPI settings, including angle sweeps that return to their starting point without cumulative crop shrinkage. At 100%, check actual source detail, detail-load latency and memory during rapid pan/zoom; an enlarged Fit preview is only a temporary loading state. Inspect metadata tags/containers independently for both default stripping and Keep metadata export. The M1 color probe must choose numerical tolerances and display-viewing conditions; tolerances are not assumed from JPEG byte equality. Later RAW tests separate decode success, sensor interpretation, baseline appearance and subjective detail quality.

Avoid flaky CI timing gates on shared runners. CI should enforce numerical correctness, allocation/queue bounds where deterministic, and build/test coverage. Compare performance on the recorded M4 reference machine and preserve measurement reports. S0 currently requires packaged native M4 evidence and automated portable checks, with rendering/VM limitations explicit. Manual Windows/Linux checks are deferred. M1 extends the native Mac journey and separately rechecks the added editor behavior on Windows/Linux; earlier skeleton success does not prove later editing/IPC features. Cross-compilation or a Linux VM alone does not prove native desktop/GPU support.

For VM checks, record guest architecture, hypervisor/runtime version, guest graphics API/adapter, software versus accelerated rendering, memory allocation and shared-folder use. An OpenGL-to-Metal virtual GPU path does not establish Vulkan support or native Linux timings. Keep the active catalog inside the guest's local filesystem rather than a host shared folder. [VM research](../research/technical-options.md#apple-silicon-and-linux-vm-testing).

## Module activation measurements

The [module design](../design/modules-and-api.md#optionality-and-performance) requires evidence before splitting basic functionality into separately loaded binaries. Logical code boundaries and user enablement do not by themselves establish lower memory use or faster launch. No Lightwell module-loading benchmark has run.

After the first use case is selected, TASK-067 compares the working built-in baseline with a focused activation prototype on the M4. Measure minimal/default configurations, disabled optional modules, enabled-but-unused modules and first use. Where worthwhile, compare lazy linked resources with separate runtime loading at equivalent functionality/output. Record module/dependency manifests, actual resource initialization, cold/warm launch, RSS/peak memory, CPU/GPU allocations, idle work and first-use latency. Use the same fixtures/build settings and the sampling/reporting rules above; note packaging size separately from resident memory.

Check that disabled modules start no workers or background jobs and allocate no processor resources. Account for shared decoder dependencies: disabling one camera family may not remove its shared library cost. Compare the saved work with loader/dispatch/IPC overhead where applicable, and report latency merely deferred to first use. Reject a claimed optimization if it changes output or skips an existing recipe effect. If the benefit is immaterial, retain the simpler built-in implementation and API boundary; the external-loader requirement remains. Product TASK-034 determines relevant user priorities, not an invented percentage improvement target. Windows/Linux portability evidence stays separate from native M4 performance.

## Growth rules

Catalog opening must not enumerate/decode all originals. Grid memory should depend on visible items and cache quotas. Import, hashing, thumbnailing and AI indexing use backpressure and resumable batches. A user switching photos cancels/deprioritizes obsolete preview requests. The app must stay interactive while exports and indexing run.

Introduce full-resolution tiling and neighborhood halos before declaring unrestricted large-image local processing. A cleanly reported resource limit is acceptable during M1 if documented; silently exhausting memory is not.
