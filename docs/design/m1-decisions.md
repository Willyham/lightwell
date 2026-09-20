# Accepted editor behavior

These accepted behaviors are planned across the [four editor milestones](history-first-roadmap.md) and editor follow-ups. Editor implementation is on hold.

| Area | Accepted behavior | Current scope |
| --- | --- | --- |
| Workspace | One workspace, centered image, collapsible controls, no library grid. Cmd/Ctrl+O imports; Cmd/Ctrl+Z and Shift+Cmd/Ctrl+Z navigate history. Fit/100%/numeric zoom are visible; Tab follows controls | Foundation, then tool-specific controls |
| Geometry | Visible composition carries with mirror/quarter-turn; locked ratio orientation swaps on a quarter-turn. Free/Original/1:1/3:2/4:3/16:9/custom ratios, no numeric crop edges initially. Space-drag pan; fine angle ±45° | Basic transforms, then crop module |
| Crop drafts | Apply commits once; Cancel discards. Option scales proportionally about the fixed center. Straightening preserves composition with necessary trimming | Crop module |
| History | Every committed image action shares durable attributed history; undo/redo and all snapshots survive restart. Preview is read-only; Restore appends an action and retains all later history. Drafts are not persistent | First milestone, used by every later tool |
| Recovery | Import by reference. Verified manual Locate retains asset identity/layers/history and rejects changed or ambiguous sources | Identity in foundation; Locate in editor follow-ups |
| Export | JPEG quality 90, native destination picker, suggested `-edited.jpg`, reject existing files/source aliases. Strip optional metadata by default; Keep metadata retains supported descriptive/capture/GPS fields with correct geometry/profile and no stale thumbnail | Editor follow-ups, after a metadata/color proof |
| Live agents | Human/agent actions share history. External commits preserve human drafts with explicit conflict/discard/reapply. Reconnect queries state; client disconnect does not cancel another client's jobs | Live JSON/IPC in foundation; draft behavior with crop; MCP adapter in follow-ups |

No private metadata round-trip, resize/batch export, future tonal/mask tools or external-loader runtime is implicitly selected. Pixel-stage coordinates and layer order are defined by the core design. Crop fitting/filter/rounding and supported metadata fields remain bounded engineering proof tasks.

Owner priorities include speed, unused-tool bloat, clearer export, filtering/tagging/collections, lazy shoot subsets, selection/stacking and bracket/panorama identification. See [workflow priorities](../decisions.md#owner-workflow-priorities); later library choices remain open.

The [history contract](../specs/edit-history.md) defines the saved-stack/undo-parent/redo-path rules. Undoing Restore returns to its preceding current state; redo reapplies it. A new edit clears shortcut redo while retaining every old entry for preview or restoration.
