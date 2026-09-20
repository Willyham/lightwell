# Active task plans

Each JSON file owns its task IDs. `TASK-001` means the first task in the file being read. Dependencies and task references never point into another task file. Tasks are listed in dependency order, numbered from 001, with per-file execution waves derived from those dependencies.

| Plan | Tasks / waves | Purpose and activation |
| --- | --- | --- |
| [S0](implementation-s0.json) | 37 / 13 | Accepted viewer and outstanding verification/maintenance follow-ups |
| [M1 — history](implementation-m1-history.json) | 9 / 7 | Layers, pixel proof, history, persistence, UI and live API; first implementation milestone |
| [M2 — transforms](implementation-m2-transforms.json) | 5 / 5 | Rotate and both reflection axes after the working history foundation |
| [M3 — modules](implementation-m3-modules.json) | 6 / 5 | Tool schemas, controls and actions after the working transform editor |
| [M4 — crop](implementation-m4-crop.json) | 6 / 6 | Lightroom-style crop/straighten module after the module host |
| [Editor follow-ups](implementation-editor-followups.json) | 6 / 4 | Export/color/metadata, Locate, MCP and full-editor verification |
| [Later extensions](implementation-extensions.json) | 1 / 1 | External-loader measurements and proof planning after a selected use case and working module host |
| [Product decisions](product-decisions.json) | 14 / 1 | Accepted owner choices and remaining product questions |
| [Lightroom research](research-lightroom.json) | 1 / 1 | Completed technical research |
| [darktable research](research-darktable.json) | 1 / 1 | Completed source-level technical research |

Implementation is on hold by owner instruction. M1's first task is locally `ready`; later milestone roots stay `pending` until activated. “Ready” describes dependencies, not permission to resume. The roadmap defines sequence using named outcomes rather than references between task files.

Use the [history-first design](../docs/design/history-first-roadmap.md) and [planning conventions](../docs/task-planning.md).

Validate every changed plan using Create Tasks, then run `cargo xtask check`. The Rust checker discovers active top-level JSON files, checks local IDs/order/dependencies, rejects links to other task plans.
