# Version notes and evidence caveats

[Knowledge base index](README.md) · Checked 2026-09-20; release notes reached **Lightroom Classic 15.5** (August 2026).

## Changes that affect technical explanations

| Period | Why it matters | Detail and evidence |
| --- | --- | --- |
| 2011–2013 research / PV2012 era | Published edge-aware filtering and image-adaptive tone controls explain historical advances, not the complete contemporary engine | [Tone controls](tone-and-color-tools.md), [local Laplacian evidence](detail-and-local-contrast.md#published-local-laplacian-foundation) |
| 2019 / 2020 | Texture and Color Grading have engineer-authored explanations | [Detail controls](detail-and-local-contrast.md), [color tools](tone-and-color-tools.md) |
| Classic 11 onward | Auxiliary catalog edit data changes backup requirements | [Catalog files](storage-and-history.md) |
| 2023 | Initial AI Denoise engineering description and its derived-DNG workflow | [RAW processing](raw-and-computational-tools.md) |
| 14.4, June 2025 | Enhance controls integrate into Develop; XMP writes change scheduling | [RAW processing](raw-and-computational-tools.md), [performance](previews-and-performance.md) |
| 14.5, August 2025 | GPU-generated previews join the older display/process/export GPU roles | [GPU and previews](previews-and-performance.md) |
| 15.0, October 2025 | Heavy edit data can require an ACR companion to XMP | [Sidecars](storage-and-history.md) |
| 15.3, April 2026 | Adobe reports interactive slider/memory improvements | [Performance](previews-and-performance.md) |
| 15.4, June 2026 | Release notes add Apple Neural Engine support to Denoise performance | [RAW processing](raw-and-computational-tools.md) |
| 15.5, August 2026 | Render to DNG adds a rendered-output operation distinct from keeping a RAW recipe | [Storage and output](storage-and-history.md) |

The recent release chronology above follows Adobe's release ledger. Availability does not quantify speed or prove equivalent numerical output across implementations. [S35: Lightroom Classic release notes](https://helpx.adobe.com/lightroom-classic/desktop/introduction-to-lightroom-classic/release-notes.html)

## Conflicting or misleading source combinations

| Issue | Evidence conflict or trap | Treatment in this knowledge base |
| --- | --- | --- |
| Neural Engine | May 2026 Enhance help says macOS Denoise does not support it; newer June release notes say it does | Prefer the newer, version-specific release note; no M4 timing inferred |
| Process version | Develop help calls PV2012 “current” while also listing later process versions | Preserve historical behavior and name the relevant PV; do not equate catalog, application and process versions |
| Enhance output | Historical engineering posts produce new DNGs; newer help integrates controls into Develop but retains some DNG language | Identify the era and avoid a blanket “all Enhance always creates / never creates a new DNG” rule |
| GPU previews | An older GPU FAQ predates preview acceleration | Read alongside the 14.5-specific preview page |
| Backup scope | Backup help opens with “catalog only”; its restore steps and FAQ also discuss auxiliary catalog data | Preserve `.lrcat-data` as edit data and separately back up originals; test restore against the installed release |
| Page timestamps | Pages dated 2025 include v15 sections; older general pages contain recently added paragraphs | Attribute claims to explicit feature/version text where possible |
| Color precision | Adobe documents module color spaces, not every internal buffer format and transfer function | Do not infer a universal bit depth, linear-light pipeline or a precise internal “Melissa RGB” implementation |
| Workflow order | A recommended editing sequence is not a trace of execution order | Separate user workflow, edit dependencies and pixel-processing order |
| SDK freshness | Official portal was accessible; detailed API reference was retrieved from a third-party mirror | Treat signatures as research evidence; confirm against an official downloaded SDK before implementation |

Underlying sources and dates are retained in the [source register](sources.md). These cautions constrain claims; they are not evidence that Adobe's current implementation is defective.

## What this research did not measure

No Lightroom installation, native GPU trace, memory profile, file-difference experiment, RAW corpus or side-by-side export was run. No preview-to-final error bound, exact slider equation, kernel radius, denoise model architecture, mask serialization format or cache dependency graph was established. No claim of Lightroom-compatible rendering follows from implementing a similarly named control.

A future update should record the installed build, process version, preferences, camera/profile, image dimensions, GPU settings and source/output hashes alongside measurements. Recheck live release notes before converting these notes into implementation requirements.
