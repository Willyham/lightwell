# Previews, loading and performance

[Knowledge base index](README.md) · Evidence checked 2026-09-20. No local Lightroom benchmarks were run.

## Several kinds of cached image data

**D.** Adobe distinguishes these resources. They should not be described collectively as “the preview.” [S09: Optimize performance](https://helpx.adobe.com/lightroom-classic/desktop/technical-support/performance-guidelines/optimize-performance-lightroom.html) [S10: Smart Previews](https://helpx.adobe.com/lightroom-classic/desktop/viewing-photos/lightroom-smart-previews.html)

| Resource | Purpose | Tradeoff |
| --- | --- | --- |
| Minimal / embedded preview | Fast initial display from camera-provided imagery | May show a different rendition from Adobe processing |
| Standard preview | Lightroom-rendered browsing image sized for display | Up-front rendering avoids later waits |
| 1:1 preview | Full-pixel Library inspection | Higher generation/storage cost |
| Smart Preview | Smaller lossy-DNG source usable for offline edits | Limited source detail; not just a finished thumbnail |
| Camera Raw cache | Reusable early-stage RAW data for Develop | Can skip initial processing; not a cache of every final recipe |
| Live rendered frame | Current image for the active view | Exact retention and invalidation policy unpublished |

**C.** Prebuilding Library previews does not establish that arbitrary future Develop edits are already computed: a browsing rendition and reusable source-stage data answer different requests.

**D.** Smart Previews are stored alongside the catalog and support disconnected editing. With the prefer-proxy setting, Develop uses them even when originals are available; Adobe documents a return to originals at 100% and full-quality final output when the original is available. Availability for a specific AI tool must be checked separately. [S10: Smart Previews](https://helpx.adobe.com/lightroom-classic/desktop/viewing-photos/lightroom-smart-previews.html) [S07: Develop module options](https://helpx.adobe.com/lightroom-classic/desktop/process-and-develop-photos/develop-module-options.html)

## Loading is a sequence of readiness states

**C.** A useful observable sequence is:

`selection acknowledged → any valid preview → current-recipe Fit preview → true-detail 100% view`

The first displayed image can be fast but stale, camera-rendered or lower resolution. “Time to image” and “time to correct image” are different measurements. RAW preview appearance changes can be expected during the transition from embedded imagery to Adobe's rendition. [S15: Color FAQ](https://helpx.adobe.com/lightroom-classic/desktop/technical-support/technical-issues/miscellaneous-issues/color-faq.html)

**P.** For Lightwell, each result should identify asset, recipe revision, source/proxy, resolution and color state. Stale results must not replace newer selections. A quick thumbnail is valuable feedback; it should not masquerade as a completed render. These are engineering implications, not claims about Lightroom's internal job objects.

## GPU support is operation-specific and versioned

**D.** Adobe exposes separate acceleration categories for display, image processing and export; configuration can disable acceleration after a capability test. The older FAQ also excludes supported VM GPU operation. Hardware presence alone does not establish which work is accelerated. [S12: GPU FAQ](https://helpx.adobe.com/lightroom-classic/desktop/technical-support/technical-issues/gpu-issues/lightroom-gpu-faq.html)

**D.** Classic **14.5** added GPU preview generation. Its documented Auto policy requires full acceleration and at least 16 GB GPU memory; On and Off overrides are available, and turning overall graphics processing off disables this option. This supersedes older blanket claims that preview building is CPU-only. The published criterion is not a recommendation to infer dedicated VRAM from an M4's unified-memory total. [S13: GPU preview generation](https://helpx.adobe.com/lightroom-classic/desktop/kb/gpu-preview-generation.html)

**D.** Classic **15.4** release notes add Apple Neural Engine support for Denoise. That supersedes a May 2026 Enhance page saying it is unsupported. Actual model/hardware routing and timings on the owner's M4 remain unmeasured. [S35: Lightroom Classic release notes](https://helpx.adobe.com/lightroom-classic/desktop/introduction-to-lightroom-classic/release-notes.html)

## Disk, cache and scheduling

**D.** Adobe recommends fast catalog/preview storage; photos, but not active catalogs, can reside on network storage. Many local corrections/history states can increase costs. [S09: Optimize performance](https://helpx.adobe.com/lightroom-classic/desktop/technical-support/performance-guidelines/optimize-performance-lightroom.html)

**D.** Preview cache limits are not strict immediate byte caps: the catalog settings page says previews newer than 30 days are retained and purging happens while idle, without a fixed schedule. [S11: Create and manage catalogs](https://helpx.adobe.com/lightroom-classic/desktop/manage-catalogs-and-files/create-catalogs.html) For XMP, the June 2025 update pauses automatic writes during import and writes the active image at ten-second intervals. This is metadata-write batching, not documented catalog transaction latency. [S36: June 2025 feature summary](https://helpx.adobe.com/lightroom-classic/desktop/help/whats-new/2025-4.html)

**C.** Separate likely cost categories before blaming a slider or language:

| Workload | Measure independently |
| --- | --- |
| Cold launch | Database opening, folder enumeration, plugin/resource initialization |
| Fast culling | Cache hit ratio, disk/decode latency, visible-item scheduling |
| First Develop visit | Source access and early processing |
| Dragging a slider | Event-to-present latency, temporary quality, queue depth |
| 100% pan/zoom | Required detail, upload and memory pressure |
| Many masks/repairs | Region size, operation count, dependency recomputation |
| Export | Full-resolution processing, resampling, encoding and writes |
| AI processing | Model loading, inference, auxiliary storage and downstream refresh |

This is a measurement taxonomy, not a recovered Lightroom profiler trace. Exact thread pools, tile dimensions, GPU residency, LRU keys, speculative prefetch and cancellation strategies are unknown.

**H.** The Halide paper explains why locality, recomputation and fusion across stages matter alongside parallelism. Its old research timings must not be republished as current Lightroom performance or proof that Lightroom now uses Halide. [S20: Halide image-processing pipelines](https://people.csail.mit.edu/jrk/halide-pldi13.pdf)
