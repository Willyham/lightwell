# Roadmap

Lightwell is built history-first: prove non-destructive state, history and the programmable API, then add tools of increasing complexity. Pillars are in [AGENTS.md](../AGENTS.md), accepted decisions in [decisions](decisions.md) and per-capability status in [features](features.md).

## Milestones

| Milestone | Result | Status |
| --- | --- | --- |
| S0 — viewer | Cross-platform skeleton that opens one JPEG at Fit | Accepted |
| M1 — history foundation | Referenced JPEG in a SQLite catalog, ordered edit layers, a pixel-change proof, persistent history with undo/redo/restore/preview, catalog reopen, desktop UI and a live JSON/IPC API | Implemented, verified on M4 |
| M2 — transforms | Exact rotate left/right, mirror horizontal and flip vertical through the same layers, history, UI and API | Implemented, verified on M4 |
| M3 — tool modules | Declarative tool interface (actions, schemas, semantic controls, validation, processing); the pixel and transform tools migrate to it with saved history unchanged | Implemented, verified on M4 |
| M4 — crop module | Lightroom-style crop and straighten as a module: free handles, ratios, straightening, Apply/Cancel, UI/API parity | Planned |

Each milestone starts from the previous working result and ends with a native M4 demonstration plus proportionate automated correctness and recovery checks. Contracts: [milestone design](design/history-first-roadmap.md), [history](specs/edit-history.md), [modules](design/modules-and-api.md), [crop](specs/single-image.md).

## Editor follow-ups

After M4: JPEG export (quality 90, no overwrites, optional metadata stripped by default with a Keep metadata option, verified color and geometry), manual Locate for moved originals, a standards-compliant MCP adapter over the existing service, and complete native packaging and performance verification. Native Windows/Linux desktop checks and manual license reviews stay deferred until the owner asks.

## Later phases

Relative priority still needs owner input; see the [open questions](decisions.md#open-product-questions).

| Phase | Scope |
| --- | --- |
| Useful small library | Multi-image import, virtualized browsing, filtering, tagging, collections, lazy shoot subsets, multi-selection and stacking, broader source recovery |
| Tonal editing and PNG | Exposure, white balance, contrast and related controls with explicit numerical and color contracts |
| Trustworthy RAW | Nikon Z6 and Fujifilm X100VI by actual recording mode; benchmark established decoders before custom work |
| Richer tools | Texture, clarity, dehaze; separately scoped masks and clone/heal |
| External modules | Measured activation costs and a separately authored module loaded through documented host APIs |

## Not in scope

Map, Book, Slideshow, Print, Web and Publish Services. Accounts, cloud sync, built-in AI chat, generative editing, a plugin marketplace, a public compatibility framework and a generalized processing graph. General bitmap layers with blend modes and reordering: a "layer" here is an ordered recipe operation.
