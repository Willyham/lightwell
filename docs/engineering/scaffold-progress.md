# S0 scaffold execution

Status: in progress, 2026-09-19. Executes the existing implementation plan; does not implement M1.

## First slice

Start TASK-002 (synthetic fixture corpus), TASK-035 (inspect the external decision gate), and TASK-037 (repository conventions). Then run the independent TASK-003 UI and TASK-036 JPEG probes. The maintained application workspace follows TASK-005 stack selection, which requires accepted TASK-023–025 product decisions. No license or target floor is silently selected.

Implement a deterministic fixture generator with an explicit Pillow version, labeled asymmetric color/detail content, EXIF 1–8, tagged/untagged sRGB, greyscale, unsupported CMYK/profile, malformed/truncated and oversized headers. Generate 24/60 MP workloads on demand outside tracked files. Record hashes and expected oriented dimensions. Tests must independently inspect headers and decoded geometry, and confirm regeneration preserves the corpus.

Repository conventions cover ignored output/private data, LF text, contributor workflow, and portable document/task checks. Checks validate both active plan shapes, dependency graphs, derived waves, file links and external completion gates. The archived plan stays untouched. A local copy of the schema permits checks without access to a personal skill installation; the checker uses Python's standard library.

## Acceptance and unresolved work

Complete each task only against its existing acceptance criteria. S0 cannot close without packaged native desktop evidence on the accepted matrix. Available host: macOS 26.5.2, arm64, M4 Pro (14 CPU / 20 GPU cores), 48 GB unified memory. Display scale/profile, storage and renderer evidence remain to measure; hardware identifiers are excluded from reports. Windows/Linux desktop sessions are not currently established.

The product gate inspection found TASK-023, TASK-024 and TASK-025 still ready/unanswered. Proposed defaults and prototype-only license deferral have been sent to the owner; no response has yet been recorded. Independent fixtures, conventions and probes may proceed.

## Probe design

Use an isolated `probes/s0` Cargo workspace, Rust 1.94.0 (available stable compiler), Iced 0.14.0 and eframe 0.33.3, each with wgpu and native file-dialog integration. These are trial pins, not a final stack decision. Use a common JPEG probe library and the same fixture. The window has only Open, status and a centered Fit surface; capture must come from the framework window renderer. Run the same source and pixel expectations in both candidates. Decode runs on a worker, with one active and one pending request and generation checks. Record native-run failure honestly if the desktop is inaccessible.

JPEG trial: `image` 0.25.9 with JPEG only, proposed 64 MP / 16384-pixel side / 128 MiB source bounds, EXIF applied once. Inspect component count before decoding to reject CMYK. Initially accept only untagged sRGB and the exact generated sRGB profile; this deliberately conservative tagged subset is experimental and must be revisited before declaring the maintained S0 input contract. Probe corrupt/truncated/profile/dimension errors, oriented quadrant colors and 24/60 MP memory/time. Renderer ICC/display behavior remains a separate native evidence requirement.

## Current outcome

TASK-002, TASK-036 and TASK-037 are complete. TASK-003 has compiled/native-render evidence with manual interaction checks outstanding. Product answers and local-bundle UI approval remain pending; TASK-035 and maintained scaffolding are not complete. See the [full probe report](s0-probe-results.md) for checks, measured values, failures and next work.

Owner follow-up: GPL-3.0-or-later selected, product TASK-024 complete. The earlier prototype-only deferral proposal is superseded. TASK-023/025 and native bundle approval remain unresolved.

Owner follow-up: initial macOS Apple Silicon / Windows x64 / Linux x64 targets and unsigned development packages accepted. TASK-023 remains in progress for OS/runtime floors, Linux window-system coverage and explicit test routes.

Owner follow-up: dark Open/Fit-only shell and previous-photo retention on failed replacement accepted. Product TASK-025 still needs the initial JPEG/profile contract; no experimental allowlist is silently promoted to accepted behavior.

Owner follow-up: standard sRGB/greyscale JPEGs, automatic orientation and explicit unsupported-profile errors accepted. TASK-025 is complete. TASK-023 platform details and native-bundle approval remain unresolved.

Owner follow-up: engineering may select and document provisional OS/runtime floors and Linux desktop configuration. Remaining platform-matrix work is technical, not an unanswered owner decision. The separate local-bundle UI action still awaits explicit approval.

Current follow-up: product TASK-023/024/025 and implementation TASK-035 complete. Owner approval for local builds/runs is recorded. Native UI interaction is waiting only on an unlocked Mac. Concrete platform baselines and missing test routes are in [platforms](platforms.md).

## sRGB profile probe follow-up design

Before stack selection, extend TASK-036's ICC recognition beyond exact fixture bytes. Trial the already-resolved moxcms 0.7.11 parser with explicit profile/TRC/CLUT limits. Accept only RGB matrix/TRC profiles whose D50 colorants and all 8-bit input transfer values match standard sRGB within documented numerical tolerances; reject LUT profiles, inconsistent CICP, malformed profiles and other gamuts. Descriptions/timestamps/manufacturer metadata must not determine acceptance. Untagged greyscale remains supported; tagged greyscale outside the declared subset is explicit unsupported-profile behavior. Test a profile serialized independently by moxcms, the existing Little CMS/Pillow fixture, benign header changes, altered gamma, wide gamut and malformed inputs. This is a follow-up experiment, not a final broad ICC claim.

Follow-up ICC experiment is implemented and unit-verified; see the updated probe report. Local execution is approved, but the next native UI attempt reported the Mac locked. Stack selection and maintained scaffolding remain downstream of the pending native interaction evidence, not a new permission request.

## Maintained scaffold and host package

The root Rust/Iced workspace now builds and runs. Six core tests verify supported/rejected inputs, EXIF orientations, source preservation, independent sRGB variants, latest-request scheduling and bounded preview dimensions. Formatting, strict Clippy and repository checks pass. Native Metal smoke validates empty/load/failure states; the unsigned host package also passed failed-replacement smoke with spaces in its evidence path. A ZIP timestamp issue from copied registry notices was fixed with ZIP timestamp clamping.

Doctor/build/develop/check/smoke/inventory/audit/package operations are implemented. TASK-039 is complete; individual behavior/evidence/platform tasks retain their acceptance requirements. CI is configured but unexecuted. License/source policy passes; two unmaintained transitive dependencies keep advisory review open. The full S0 gate remains incomplete. See scaffold-commands.md and dependency-review.md for reproducible commands and current limitations.
