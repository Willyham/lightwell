# darktable technical research plan

Status: completed 2026-09-20. Independent documentation work, tracked in [the darktable research plan](../../../tasks/research-darktable.json), locally numbered TASK-001 after the concurrent plan-ID migration. No implementation milestone changes.

## Scope and method

Build a companion to the Lightroom knowledge base covering non-destructive storage/history/sidecars; pixelpipe execution and ordering; scene-referred color; previews, caches, OpenCL, tiling and responsiveness; tonal, detail, RAW, masks and geometric tools; module contracts, Lua and CLI; and implications for Lightwell.

Use the user-provided upstream repository, pin a released tag and full commit, and inspect the actual implementation. Separate source-confirmed facts from user-manual behavior, conceptual models, runtime hypotheses and proposals. Link source paths and meaningful symbols/line anchors. Label development-only material rather than presenting master as the release. Do not build or install darktable, run an upstream script, touch a photo collection or claim measured image/performance results.

## Acceptance

- Navigable Markdown chapters and an annotated source register with retrieval date and immutable source revision.
- Trace how a settings change reaches history, pixelpipe invalidation, processing, blending and display/export.
- Explain actual algorithms for representative contrast, local-contrast/clarity, texture/detail, exposure, tone mapping, sharpening and denoise controls; distinguish them from Lightroom's names and semantics.
- Document storage/recovery, color domains, region-of-interest processing, caches and CPU/GPU boundaries with evidence.
- Record extension/automation limits, version differences, open questions and concrete future experiments.
- Preserve Lightwell decisions and original-file policy; no application changes, module adoption or feature commitments implied.
- Validate changed task plans, local links, citations/source anchors and repository checks; report any unrelated concurrent failures honestly.

## Open questions

The public code makes algorithms inspectable but does not establish M4 performance, installed-build configuration, exact CPU/GPU parity or quality on the owner's camera modes. Full dependency and asset license reviews remain deferred; reading GPL source is not a completed reuse audit.

## Completion and verification

Delivered 15 Markdown documents, approximately 13,500 words, with 81 annotated source entries spanning 65 pinned repository files and five official manual pages. The baseline is darktable 5.6.1 at commit `03179f8e080aa9cedebfe14b098b7ba88940a292`. The [index](README.md) provides question-based navigation; the [source register](sources.md) provides reading routes and immutable implementation references.

Checks performed on 2026-09-20:

- Verified checkout identity and every registered source symbol/line anchor against the pinned files. Checked all 231 pinned source-link occurrences and all local document paths/heading fragments. No unresolved authoring tokens or unbalanced code fences remained.
- Validated the completed darktable research plan and all ten active JSON plans independently with the Create Tasks validator. Preserved the concurrent change to plan-local task numbering; this research is now `research-darktable` / `TASK-001` rather than its earlier global ID.
- Checked whitespace/newlines in all new Markdown documents and ran `git diff --check` for the integration paths.
- Ran `cargo xtask check`. The first attempt during the concurrent plan split could not find a moved file; a later run reached the old global-ID overlap rule. After the checker changed to plan-local IDs, the latest run stopped at an existing roadmap link to removed `tasks/implementation-m1.json` in `docs/task-planning.md`. This is an unfinished concurrent roadmap migration, not a passing repository-wide result or a darktable research-link failure. No unrelated checker/roadmap migration was changed to suppress it.

No darktable build, image export, AI inference, GPU measurement or camera qualification was performed. Source inspection supports the documented algorithms and architecture; proposed experiments remain unexecuted. Application behavior and accepted Lightwell decisions are unchanged.
