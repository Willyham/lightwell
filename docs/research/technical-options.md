# Technical options and evidence

Status: research and recommendations, checked 2026-09-19. No comparative benchmarks have been run. Upstream capabilities are distinguished below from our engineering judgments. Pin and recheck concrete dependency versions during M0; moving documentation can describe unreleased code.

## Desktop interface options

“Native” has three separate meanings: an OS executable, OS-provided widgets, and direct GPU rendering. A native executable with custom GPU controls satisfies the proposed direction without requiring three separate widget implementations. None of the frameworks below guarantees attractive design or fast photo processing by itself.

| Candidate | Evidence and fit | Principal uncertainty | Recommendation |
| --- | --- | --- | --- |
| Rust + Iced + wgpu | Cross-platform, MIT, reactive messages, custom widgets; published shader widget exposes wgpu integration | Upstream calls it experimental; verify accessibility, focus/IME, virtualized layouts, color output, and shared GPU resources in the pinned release | Leading trial |
| Rust + egui/eframe + wgpu | Native integration and immediate-mode UI; readily customizable; MIT/Apache-2.0 | Test polished layout/input and large-grid behavior; measure idle redraw behavior rather than assuming immediate mode burns CPU continuously | Small comparison prototype |
| Rust + Slint | Declarative UI; documented software and GPU renderer options | Custom photo-surface integration and license choice; royalty-free terms are distinct from the GPL open-source option | Consider if its authoring model or tooling wins the UI trial |
| C++ + Qt Quick, or Rust core with a narrow C++ bridge | Scene graph uses native graphics APIs; extensive desktop platform facilities | Binding/build complexity with Rust; deployment size; module-by-module LGPL/GPL obligations | Serious fallback if Rust UI candidates fail required UX |
| Rust + GPUI | GPU-rendered framework used by Zed; current platform documentation covers macOS, Linux, and Windows | Pre-1.0 churn and ties to Zed; investigate custom image-pipeline integration rather than infer it from text-editor speed | Credible candidate, not first trial |
| Tauri + web UI + native core | Uses system WebViews and supports Rust integration | Photo-surface GPU ownership, interop/copying, and differences between platform WebViews need proof | Viable if web contributors/design velocity outweigh a unified native render path |
| Flutter + native image core | Official desktop support includes all three target OSes | Adds Dart and a native interop boundary; verify custom texture and color paths | Viable, but no clear benefit for this project's small core yet |

Primary sources: [Iced repository](https://github.com/iced-rs/iced), [published Iced shader API](https://docs.rs/iced/latest/iced/widget/shader/index.html) (reported 0.14.0 when checked), [egui](https://github.com/emilk/egui), [Slint renderers](https://docs.slint.dev/latest/docs/slint/guide/backends-and-renderers/backends_and_renderers/), [Slint license options](https://www.slint.dev/community), [Qt scene graph](https://doc.qt.io/qt-6/qtquick-visualcanvas-scenegraph.html), [Qt licensing](https://doc.qt.io/qt-6/licensing.html), [GPUI platform README](https://github.com/zed-industries/zed/blob/main/crates/gpui/README.md), [GPUI crate license](https://github.com/zed-industries/zed/blob/main/crates/gpui/Cargo.toml), [Tauri](https://v2.tauri.app/start/), [Flutter desktop](https://docs.flutter.dev/platform-integration/desktop).

Our preference for Iced is an architectural judgment: Rust throughout and direct access to the same GPU abstraction used by the image engine are promising. It is not a finding that Iced is faster or more polished than Qt, Slint, GPUI, or a WebView. Build only a window, photo surface, crop overlay, slider/input field, and a long list in the trial. If essential accessibility or display-color behavior cannot be delivered without maintaining a framework fork, reconsider the choice.

For the first comparison, use the same photo, window dimensions, controls, GPU workload, and release-build settings. Record startup, resident memory, idle CPU, input-to-preview latency, GPU copies, build/package size, and missing platform behavior. UI toolkit dependencies must remain outside the domain model.

## Apple Silicon and Linux VM testing

The owner selected an M4 MacBook Pro as the first target. Develop and benchmark natively on macOS arm64, using Metal through the portable renderer; retain Windows/Linux build coverage and later native GPU checks.

[Try Omarchy](https://github.com/omacom/try-omarchy) currently documents Apple Silicon/macOS 15+ support and a QEMU/virgl/ANGLE graphics path translating guest OpenGL through Metal. It is a plausible ARM64 Linux functional-test candidate, not evidence that our wgpu feature set or native Linux performance works. Verify the pinned runtime's actual adapter/features. [Omarchy's September 2026 announcement](https://omarchy.org/news/2026/09/introducing-omarchy-m/) links this VM separately from native Mac installation work; no repartitioning is needed for our proposed test approach.

[UTM's QEMU display documentation](https://docs.getutm.app/settings-qemu/devices/display/) describes experimental VirGL acceleration with application compatibility limits. Neither VM route should be assumed to provide native Vulkan capabilities. If the chosen app backend cannot run in the VM, record that limitation and use a native Linux GPU machine for renderer validation. VM functional checks and headless CI still have value. No VM has been installed or tested in this project.

## Rendering and native core

[wgpu](https://wgpu.rs/) provides native graphics and compute across Metal, Vulkan, DirectX 12, and other backends. We propose Rust for domain logic and scheduling, with wgpu/WGSL for interactive image presentation and later processing. Backend support does not imply every GPU supports every format, dimension, or compute feature; probe capabilities and budget resources.

Start with CPU decode and a CPU reference geometry renderer, upload a bounded preview once, and transform it on the GPU while dragging. Avoid decoding, resizing, or copying a full-resolution image for each pointer event. Use the CPU path for correctness comparisons and headless export initially; share coordinate and color definitions. Shader processing and CPU export must be checked for equivalent results within declared tolerances.

Do not build an entire GPU image graph for the first crop tool. Tile/pyramid processing becomes essential as image size and neighborhood operations grow. Selective invalidation, memory accounting, and cancellation matter earlier than exotic kernel optimization.

## Image processing and RAW

| Component | Proposed role | Limits to account for |
| --- | --- | --- |
| [image-rs/image](https://github.com/image-rs/image) | Initial JPEG decode/encode behind an adapter; add PNG later | File decoding is not complete color management or high-quality RAW development; verify metadata/profile access in the chosen version |
| [Little CMS](https://littlecms.com/color-engine/) | ICC input/output transformations and CPU color reference; MIT license | Does not by itself obtain each monitor's current profile or establish the OS surface/compositor color contract |
| [LibRaw](https://www.libraw.org/about) | Preferred RAW decoding trial: pixels, sensor metadata, embedded previews | LGPL-2.1/CDDL-1.0 choice; upstream explicitly places production-quality rendering outside its scope |
| [RawSpeed](https://github.com/darktable-org/rawspeed) | Alternative to benchmark when decompression/unpacking is a measured bottleneck | Supplies sensor data; does not demosaic or perform color correction; qualify exact-camera coverage, integration and license at the selected revision |
| [Rawler/DNGLab](https://github.com/dnglab/dnglab) | Alternative Rust RAW decoder to investigate if exact-camera support or integration justifies it | LGPL-2.1 repository; supported extensions do not establish complete rendering quality or recording-mode coverage |
| [libvips](https://www.libvips.org/) | Possible future thumbnail/batch processing helper | Additional native dependency; benchmark a concrete workload before adding a second image-processing system |

LibRaw's [camera list](https://www.libraw.org/supported-cameras) lists both the owner's original **Nikon Z6** and **Fujifilm X100VI**. The page qualifies coverage by enabled build features; actual recording modes still require fixtures. Its HE/HE* exclusions name other Nikon models, including Z6 III, and should not be presented as an exclusion for the owner's original Z6. Track exact firmware/sample, compression, bit depth, and sensor layout. An embedded JPEG is useful for browsing but is not proof of RAW development. [LibRaw API overview](https://www.libraw.org/docs/API-overview.html).

The owner is open to a new RAW library if existing options fail. Recommended escalation: (1) measure I/O, unpack/decompression, demosaic, color and preview upload independently on actual Z6/X100VI files; (2) compare suitable LibRaw/RawSpeed/Rawler paths at equivalent output stages and quality; (3) fix scheduling/copies or contribute a targeted optimization/support patch; (4) consider a fork or narrowly scoped new decoder only for a reproducible gap. Set target latency/memory and a correctness corpus first. Age or implementation language alone is not evidence of poor performance, and replacing a decoder cannot fix an unrelated demosaic or preview bottleneck. No comparative decoder benchmarks have run.

For RAW, the missing work includes black/white levels, mosaic interpretation, demosaic quality (including relevant Fujifilm X-Trans sensors), camera-to-working-space conversion, white balance, highlight behavior, and a pleasing default tone rendition. Lens corrections and sharpening/noise reduction may become practical RAW requirements; decide them explicitly rather than promise a professional RAW workflow from decoding alone.

JPEG temperature is a creative adjustment to already rendered pixels; RAW white balance acts on camera data. Do not promise identical Kelvin semantics or recovery latitude. Familiar slider names also do not imply Adobe-identical algorithms or Lightroom edit compatibility.

## Catalog and storage

[SQLite](https://www.sqlite.org/whentouse.html) is the proposed embedded catalog store, with indexed metadata, edit recipes, and revision records. Store originals externally and disposable previews separately. SQLite is a plausible fit, but our million-photo workloads need their own query and I/O measurements.

Use short transactions and one application-level writer. [WAL documentation](https://www.sqlite.org/wal.html) explains concurrent readers, checkpointing, and the shared-memory restriction: keep the active catalog on a supported local filesystem. Originals may later live on removable disks or NAS. A live catalog inside a network share or cloud-sync folder is not an initial supported configuration. Use SQLite's [backup API](https://www.sqlite.org/backup.html) for consistent snapshots rather than copying only a live database file.

## Agent and extension interfaces

The command API is the application boundary; MCP is an adapter. Use JSON-schema-described operations with state queries, bounded previews, structured errors, revision checks, and jobs. UI code calls the same service directly without JSON serialization overhead. The owner brought **MCP and simultaneous GUI/agent control into M1** on 2026-09-19; implement local IPC and shared-state coordination explicitly rather than maintaining separate CLI and GUI catalogs.

The official [MCP tools specification](https://modelcontextprotocol.io/specification/2026-07-28/server/tools) supplies tool schemas and structured results. [stdio transport](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/stdio) fits a local client-launched process. Prefer the [official Rust SDK](https://github.com/modelcontextprotocol/rust-sdk), verifying its actual released protocol support against intended clients. Do not mix protocol examples from different revisions or introduce a hosted service to serve a desktop app.

For external extensions, start with presets and workflow commands; consider capability-limited WebAssembly only after a real extension needs it. [Wasmtime's security model](https://docs.wasmtime.dev/security.html) provides isolation machinery, but the host controls granted access. Resource limits and a narrow API still need to be designed. WASM is not our assumed execution path for every image kernel. Third-party native binaries or GPU shaders require a separate trust, performance, and failure-isolation design.

## Reuse versus a new application

[darktable](https://www.darktable.org/about/) already combines catalogs, non-destructive processing, modular operations, and floating-point image work under GPLv3-or-later. [RawTherapee](https://www.rawtherapee.com/) provides a substantial cross-platform RAW processing system under GPLv3. Both are valuable behavior and image-quality references.

An existing-engine fork could reach sophisticated RAW output sooner. Its cost is inheriting a large processing/UI architecture and license obligations, and still needing a coherent command interface. A fresh core makes the proposed interaction model easier to control, but takes on significant image-science work. Our recommendation is a new small application using established libraries, with an explicit owner decision about whether early RAW quality warrants reusing a larger engine. Do not copy GPL implementation into a differently licensed core without resolving compatibility.

## License decision

The owner clarified an **open-source-only preference** on 2026-09-19. Our leading recommendation is **GPL-3.0-or-later** for Lightwell, with compatible open-source dependencies and bundled/official extensions. Formal selection and application of the license remain in M0 after checking the configured build. The earlier alternatives remain here as research history:

- **Apache-2.0** is a permissive candidate when proprietary forks and integrations are acceptable; its text includes copyright and patent grants. [License](https://www.apache.org/licenses/LICENSE-2.0).
- **MPL-2.0** is a candidate when distributed changes to covered files should remain open while separate files can use other licenses. It does not require every file in a larger application to become open. [Mozilla FAQ](https://www.mozilla.org/en-US/MPL/2.0/FAQ/).
- **GPLv3** is the direction to evaluate for broader reciprocity and direct reuse of GPL photo engines. Plugin compatibility depends on the actual integration; a process or WASM boundary is not an automatic licensing exception. [GNU license text](https://www.gnu.org/licenses/gpl.en.html).

Apache/MPL are no longer the leading project-license choices given that preference, although compatible permissively licensed dependencies remain usable. Open-source distribution policy does not require replacing macOS/Windows system APIs. For Slint, evaluate its GPL route; avoid making a proprietary license essential to building Lightwell.

GPL covers distribution of the program and covered derivative work; private modifications need not be published, and commercial use is permitted. It also does not automatically require every independent program calling a public API to adopt GPL. Plugin obligations depend on how the software combines; requiring official extensions to be open source is a project policy as well as a dependency-license question. [GNU GPL text](https://www.gnu.org/licenses/gpl.en.html), [GNU licensing FAQ](https://www.gnu.org/licenses/gpl-faq.en.html).

Before distributing a build, inspect the exact native libraries, optional decoder packs, UI modules, and image/profile assets it contains. Native binaries do not have to be single statically linked files; app bundles and shared libraries may be the appropriate distribution shape. No license file has been applied by this planning update.
