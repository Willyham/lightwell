# Lightroom Classic technical research plan

Status: research in progress. Requested 2026-09-20. This is a documentation investigation, not authorization to implement photo tools or change Lightwell's milestones.

## Scope and method

Build a navigable knowledge base covering non-destructive storage, history and sidecars; Camera Raw rendering and color; previews, loading and CPU/GPU performance; tonal/detail/color tools; geometry, masks and repair; public SDK boundaries; and implications for Lightwell. Interpret the user's “contract” example as Contrast.

Use Adobe product documentation, first-person Adobe engineering explanations, public specifications/SDKs, and original research papers. Distinguish documented product behavior, historical/version-specific evidence, conceptual mathematics and unverified implementation hypotheses. Record URLs, retrieval date and limitations. Do not treat papers or patents as proof of shipping algorithms. Do not copy full source articles or proprietary code.

## Acceptance

- An index links focused Markdown chapters and an annotated source register.
- Core claims carry source links; exact proprietary details that cannot be verified are stated as unknown.
- Contrast, Clarity and Texture receive an explicit comparison, along with other major Develop tool families.
- Storage distinguishes original image samples from container metadata, durable edits from disposable caches, and current state from history.
- Rendering distinguishes user edit order from processor order and identifies proxy/full-resolution/color boundaries.
- Performance separates browsing, interactive edits, batch work and inference; no Lightroom benchmark is claimed without measurements.
- Current documentation is checked for version changes, especially AI data/sidecars and Enhance behavior.
- Lightwell implications remain proposals or references to existing accepted constraints. No feature, migration or algorithm equivalence is silently accepted.
- Both active task plans validate; project documentation/link checks pass or limitations are recorded.

## Unresolved questions

Adobe's exact filter coefficients, adaptive tone functions, processing graph, cache contents/keys, scheduling and GPU kernels may remain unpublished. Capture research gaps and proposed reproducible experiments rather than inventing these details. This research does not run Lightroom, inspect private catalogs, benchmark hardware or audit licensing.
