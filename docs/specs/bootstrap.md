# First end-to-end build: image-loading skeleton

Status: **maintained S0 in progress; macOS viewer and initial tooling verified**. See [probe evidence](../engineering/s0-probe-results.md). On 2026-09-19 the owner replaced the full editor as the first build with a cross-platform skeleton that loads an image. This is **S0**, before the previously agreed **M1 editor**. All work remains v0.

## Outcome

Build a small desktop application for macOS, Windows and Linux that opens one local JPEG and displays it correctly at Fit. The M4 MacBook Pro remains the first development machine. S0 is complete only when the agreed platform matrix has packaged launch-and-load evidence, not just successful compilation.

JPEG follows the existing JPEG-first requirement. The owner accepted a deliberately documented input subset: 8-bit RGB/greyscale JPEG, EXIF orientations 1–8, untagged-as-sRGB and supported standard tagged sRGB. The stack decision must establish a defensible recognition/display path; the experimental exact-profile allowlist alone does not satisfy this accepted scope. Reject unsupported color profiles/modes explicitly rather than silently display incorrect pixels. Broader ICC conversion and professional display validation remain M1 work unless the selected stack supplies them cleanly in S0.

## User journey

Owner-approved shell: dark background, one Open image button, automatic Fit, brief loading/error feedback, and retention of the last successful photo on replacement-open failure.

1. Launch a native application into a deliberate empty state with an **Open image** action.
2. Choose a local JPEG through the platform file picker. The standard Open shortcut invokes the same action. Cancel leaves the current state unchanged.
3. Show loading feedback while background decoding runs, then show the complete, correctly oriented photo with preserved aspect ratio and no stretching.
4. Resize the window; Fit recomputes against the available canvas and display scale.
5. Open another file. A newer request takes precedence; an obsolete decode must not overwrite it. Unsupported, missing, unreadable or malformed input produces a useful error and leaves the last successful photo intact when one exists.
6. Close the application cleanly, including while an image is loading. Source bytes remain unchanged.

Opening is a transient session operation, not catalog import. S0 does not persist photo identity or edits. The skeleton has no crop, rotate, flip, sliders, export, undo, catalog database, filmstrip, RAW support or plugin system. The agreed interactive zoom/pan/percentage controls belong to M1; S0 only fits the image automatically. Do not add placeholder controls for future tools.

## Implementation boundary

The UI and automated launch/test entry points call the same open-image service. Keep decode and file I/O off the UI thread, use checked dimensions and explicit resource limits, and discard stale generations. Keep the source adapter separate from the window framework. Start with only modules that have real work; do not create empty catalog, plugin or MCP subsystems.

The initial Rust recommendation remains provisional. A focused Iced/egui comparison should test an empty window, photo surface, file opening, resize/DPI behavior and a capture route. It does not require crop tools or the completed M1 color/geometry experiments. Record the chosen stack, exact versions and native prerequisites before scaffolding the maintained application.

GPU initialization failure must produce an actionable diagnostic. A software-rendered test may establish functional behavior if labelled; it cannot establish hardware GPU performance. The initial display contract must correctly present the declared sRGB subset without double color conversion. Do not describe S0 as a color-accurate professional editor.

## Platform gate

| Accepted target | Proposed first artifact format | Required S0 evidence |
| --- | --- | --- |
| macOS arm64 | App bundle plus archive | M4 native launch, file dialog, image display, resize/high-DPI and clean shutdown |
| Windows x64 | Executable with required runtime files in an archive | Native Windows launch, dialog, load/render and shutdown; architecture/runtime details recorded |
| Linux x64 | Relocatable directory/archive with declared runtime requirements | Linux desktop session launch, dialog, load/render and shutdown; chosen distribution and X11/Wayland path recorded |

The owner accepted macOS Apple Silicon, Windows x64 and Linux x64 with unsigned development packages. Exact artifact formats, OS/runtime floors and Linux window-system coverage remain to finalize; accepted targets are not already verified support. Linux arm64 is useful for an M4-hosted VM but does not replace testing the selected x64 artifact. A Windows ARM guest running an x64 binary must be labelled as emulated, not native x64. A VM can establish guest functional behavior; record its graphics adapter and software/accelerated path. Native hardware performance remains separate evidence.

No paid signing, store submission, public release, auto-updater, elaborate installer or VM installation is part of this planning change. S0 packages may be unsigned development artifacts with launch requirements documented. Missing machines or desktop sessions remain outstanding checks rather than passing results. Omarchy remains an optional Linux VM candidate, not a requirement for the first build.

## Agent verification

Provide a development smoke mode that can launch with a fixture path, isolated application-data directory, deterministic window size and an evidence output directory. These are proposed interfaces, not commands that exist today. The smoke controller must use the production open-image path; it must not synthesize successful state or render an unrelated test image.

For each run produce:

- A machine-readable result with pass/fail/unsupported status, stage, source fixture identifier, dimensions/orientation, render generation and failure reason.
- Structured JSONL logs with startup, file request, decode, upload/render-ready, shutdown and error events; include timings and build/platform/backend information.
- A PNG of the actual application content, tied to the requested image generation after rendering completes, plus a state snapshot. Label renderer capture versus OS-window capture distinctly.
- Test output and a Markdown reproduction summary with the exact invocation, artifact locations and platform limits.

An offscreen frame is useful for renderer correctness but is not proof that a visible OS window, file dialog, focus or compositor works. Add a native visible-window smoke check on each target. Never report a screenshot as taken when capture is unsupported or no graphical session is available. Do not use fixed sleeps as readiness checks: wait for the intended generation/state with a bounded deadline, then retain failure evidence.

Use synthetic/licensed fixtures for CI screenshots and logs. Routine logs omit private source paths and photo metadata; an explicitly selected local diagnostic run may include them. No automatic telemetry or screenshot uploads. The detailed artifact and tooling contract is in [development and verification](../engineering/development.md).

## Acceptance

S0 needs reproducible checkout/setup instructions; formatting/lint/build checks; packaged binaries for the accepted OS/architecture matrix; valid/invalid/repeated-open tests; observed native desktop behavior; source preservation; and retained logs/screenshots/results. Record optimized-build startup, first-image latency, idle behavior and peak memory on the M4 as an initial baseline. Numerical M1 budgets remain provisional; S0 still requires bounded allocations/queues, responsive loading and no continuous redraw loop when idle.

The full editor cannot be marked delivered by S0. Catalogs, geometry, export and live IPC/MCP resume through the separate M1 gate and existing [single-image specification](single-image.md).
