# Source register and code-reading map

[Knowledge base index](README.md) · Retrieved/inspected **2026-09-20**.

## Provenance

Primary repository: [darktable-org/darktable](https://github.com/darktable-org/darktable). Release: [5.6.1](https://github.com/darktable-org/darktable/releases/tag/release-5.6.1). All implementation links below use commit `03179f8e080aa9cedebfe14b098b7ba88940a292`; manual pages use the official 5.6 documentation. No forum claim is used as evidence of an algorithm.

Code was inspected in a temporary shallow checkout without fetching dependencies or executing upstream code. Line anchors are calculated against that exact checkout and verified locally; symbols provide a second lookup route. The research does not mirror the repository or depend on temporary paths for navigation.

## Suggested reading routes

| Goal | Reading sequence |
| --- | --- |
| Follow one edit | `develop.c` history update → `pixelpipe_hb.c` synchronization/evaluation → `pixelpipe_cache.c` → module `commit_params`/`process` → `blend.c` |
| Understand persistence | `database.c` library/data schema → `develop.c` history persistence → `exif.cc` XMP serialization |
| Understand scale and memory | `mipmap_cache.h/.c` → ROI callbacks → `tiling.c` → mode-specific module readiness and buffer allocations |
| Implement a small operator | `iop_api.h` → `exposure.c` → introspection and loader; inspect color/ROI/blending requirements before generalizing |
| Understand contrast/detail | `colorbalancergb.c`, `sigmoid.c` → `bilat.c` + `locallaplacian.c` → `atrous.c` + `eaw.c` → `diffuse.c` |
| Understand RAW | `imageio.c` loader dispatch → `rawprepare.c` → `highlights.c` → `demosaic.c` → `denoiseprofile.c` |
| Understand geometry/repair | `crop.c`, `ashift.c`, `interpolation.c` → mask/blend host → `retouch.c` + `heal.c` |
| Understand AI boundaries | `DefineOptions.cmake` → `AI.md`/`AI_Tasks.md` → backend + preparation code → `neural_restore.c` or object-mask finalization |
| Evaluate automation | `lua/image.c`, `lua/gui.c`, `dbus.c`, `cli/main.c`; verify actual runtime coverage before claiming parity |

## Annotated references

Each entry identifies a useful function/section, not necessarily every supporting line in a file. Nearby callers and branches were inspected where needed. Multiple anchors into a file are separate reading entry points, not independent corroborating sources.

| Reference | Evidence | Why read it |
| --- | --- | --- |
| [5.6.1 release notes](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/RELEASE_NOTES.md#L1) | Pinned source | Release scope, preview regression fix and upgrade limits. |
| [Workflow defaults](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/data/darktableconfig.xml.in#L3898) | Pinned source | The release defaults to scene-referred sigmoid; filmic and AgX are alternatives. |
| [Catalog schema](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/common/database.c#L3330) | Pinned source | Library records, history, masks, order and history hashes; implementation schema, not an external API. |
| [Shared data schema](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/common/database.c#L3138) | Pinned source | Styles, presets and tags in data database. |
| [Database ownership](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/common/database.c#L4377) | Pinned source | Startup locking and schema handling; not a concurrent SQL integration contract. |
| [XMP implementation](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/common/exif.cc#L5128) | Pinned source | Serialization and restoration of processing/history/mask data. |
| [Sidecar writer](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/common/exif.cc#L6062) | Pinned source | Checksum comparison and sidecar write behavior. |
| [History updates](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/develop/develop.c#L1176) | Pinned source | Coalescing, history truncation and pipeline change flags. |
| [History reconstruction](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/develop/develop.c#L1614) | Pinned source | Reset module defaults then replay parameter states to history_end. |
| [History persistence](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/develop/develop.c#L1769) | Pinned source | History/order/hash writes and transaction wrapper. |
| [View jobs and invalidation](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/develop/develop.c#L275) | Pinned source | Separate worker queues and dirty/timestamp updates. |
| [Pipe synchronization](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/develop/pixelpipe_hb.c#L665) | Pinned source | History settings committed into per-pipe module pieces. |
| [Pixelpipe evaluator](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/develop/pixelpipe_hb.c#L1757) | Pinned source | Recursive ROI requests, cached processing, color conversion and blending. |
| [OpenCL restart path](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/develop/pixelpipe_hb.c#L3189) | Pinned source | Late errors can restart processing on CPU. |
| [Pixelpipe cache keys](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/develop/pixelpipe_cache.c#L103) | Pinned source | Cumulative state/profile/ROI-sensitive hashing. |
| [Pixelpipe cache storage](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/develop/pixelpipe_cache.h#L33) | Pinned source | Host buffer cache and resource accounting. |
| [Mipmap/source buffers](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/common/mipmap_cache.h#L28) | Pinned source | Thumbnail levels, float preview/full input and request modes. |
| [Mipmap cache implementation](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/common/mipmap_cache.c#L709) | Pinned source | Disk/memory cache, loading and generation paths. |
| [Tiled processing](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/develop/tiling.c#L592) | Pinned source | Memory factors, overlap and alignment; inspect callback eligibility too. |
| [Processing order](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/common/iop_order.c#L298) | Pinned source | Explicit RAW/JPEG order tables and migration/reordering logic. |
| [IOP callback contract](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/iop/iop_api.h#L316) | Pinned source | Processing, geometry, introspection and lifecycle declarations. |
| [Developer module guide](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/dev-doc/IOP_Module_API.md#L1) | Pinned source | Parameter/runtime/GUI state separation; checked against implementation. |
| [Developer pixelpipe guide](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/dev-doc/pixelpipe_architecture.md#L1) | Pinned source | Architecture orientation; source is authoritative for fine details. |
| [Introspection guide](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/dev-doc/introspection.md#L1) | Pinned source | Generated parameter metadata; broad Lua claims need binding verification. |
| [IOP loader](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/develop/imageop.c#L305) | Pinned source | Callback loading, introspection, parameter preparation and migrations. |
| [Native module discovery](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/common/module.c#L27) | Pinned source | Scans installed module directory; not a sandboxed extension system. |
| [IOP build rules](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/iop/CMakeLists.txt#L39) | Pinned source | Image operations built as native loadable modules. |
| [Exposure](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/iop/exposure.c#L468) | Pinned source | Black subtraction and stop-based scaling; automatic modes can change effective EV. |
| [Legacy contrast/brightness/saturation](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/iop/colisa.c#L179) | Pinned source | Explicit Lab contrast/brightness lookup tables; deprecated module. |
| [Color balance RGB](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/iop/colorbalancergb.c#L579) | Pinned source | Tonal masks, chroma/vibrance, grading and gray-pivot contrast. |
| [Filmic RGB](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/iop/filmicrgb.c#L888) | Pinned source | Log encoding, spline rendering and multiple retained algorithm versions. |
| [Sigmoid](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/iop/sigmoid.c#L300) | Pinned source | Generalized log-logistic curve and parameter solving. |
| [AgX](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/iop/agx.c#L1326) | Pinned source | Rendering-primary transforms, gamut handling, tone mapping and look. |
| [Tone equalizer](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/iop/toneequal.c#L1167) | Pinned source | Edge-aware luminance mask and smoothly varying exposure gains. |
| [Local contrast](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/iop/bilat.c#L293) | Pinned source | Bilateral-grid or local-Laplacian dispatch; Laplacian path disallows tiling. |
| [Local Laplacian implementation](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/common/locallaplacian.c#L313) | Pinned source | Scalar remap, six intensity samples and pyramid interpolation/reconstruction. |
| [Contrast equalizer](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/iop/atrous.c#L350) | Pinned source | Edge-aware wavelet detail processing and scale controls. |
| [Edge-aware wavelet helper](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/common/eaw.c#L111) | Pinned source | Decomposition and immediate accumulation reduce stored detail buffers. |
| [Diffuse or sharpen](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/iop/diffuse.c#L1340) | Pinned source | Iterative multiscale diffusion; explicit fast-mode bypass. |
| [Sharpen](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/iop/sharpen.c#L250) | Pinned source | Gaussian unsharp mask on lightness, thresholding and scale handling. |
| [Haze removal](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/iop/hazeremoval.c#L559) | Pinned source | Atmospheric estimate, dark channel, guided transmission and inversion. |
| [Input color profile](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/iop/colorin.c#L80) | Pinned source | Camera/input transforms and default working profile. |
| [Output color profile](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/iop/colorout.c#L488) | Pinned source | Fast matrix/curve path versus Little CMS; proof/export behavior. |
| [Color calibration](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/iop/channelmixerrgb.c#L104) | Pinned source | Chromatic adaptation, channel mixing and profiling. |
| [Image loader routing](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/imageio/imageio.c#L201) | Pinned source | Magic signatures, RawSpeed/LibRaw paths and fallback. |
| [RAW normalization](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/iop/rawprepare.c#L46) | Pinned source | Per-channel black/white levels, sensor crop and gain maps. |
| [Demosaic](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/iop/demosaic.c#L53) | Pinned source | Bayer/X-Trans method selection, dual paths and reduced-resolution behavior. |
| [Highlight reconstruction](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/iop/highlights.c#L59) | Pinned source | Several sensor-stage algorithms, ROI needs and fast-mode choices. |
| [Profiled denoise](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/iop/denoiseprofile.c#L69) | Pinned source | Poisson/Gaussian parameters, variance stabilization, NLM and wavelets. |
| [Camera noise profiles](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/data/noiseprofiles.json#L3550) | Pinned source | Nikon Z 6 and Fujifilm X100VI entries exist; not a recording-mode test. |
| [Build feature switches](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/DefineOptions.cmake#L26) | Pinned source | Optional AI is off by default in source configuration. |
| [AI architecture guide](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/dev-doc/AI.md#L1) | Pinned source | ONNX backend, providers, model registry and module boundaries. |
| [AI task contracts](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/dev-doc/AI_Tasks.md#L1) | Pinned source | Bayer/linear/RGB data paths, inference tiling and derived outputs. |
| [ONNX/CoreML backend](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/ai/backend_onnx.c#L1714) | Pinned source | Runtime provider attachment and compute-unit options. |
| [Neural restore integration](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/libs/neural_restore.c#L998) | Pinned source | Derived outputs, import and metadata/group handling. |
| [Object segmentation](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/common/ai/segmentation.c#L1138) | Pinned source | Embedding cache, prompt refinement and reset lifecycle. |
| [AI mask persistence](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/develop/masks/object.c#L94) | Pinned source | Object-mask implementation and stored mask data; model assets separate. |
| [Shared blending](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/develop/blend.c#L458) | Pinned source | Drawn/parametric/raster masks, blending domain and ROI constraints. |
| [Mask host](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/develop/masks/masks.c#L30) | Pinned source | Forms, coordinate transformations, storage and common mask operations. |
| [Crop](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/iop/crop.c#L463) | Pinned source | Coordinate mapping, output bounds and ROI callbacks. |
| [Rotate and perspective](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/iop/ashift.c#L1001) | Pinned source | Geometric transform, inverse sampling and interpolation. |
| [Lens correction](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/iop/lens.cc#L73) | Pinned source | Lensfun and embedded-metadata methods. |
| [Resampling kernels](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/common/interpolation.c#L161) | Pinned source | Bilinear, bicubic and Lanczos implementations. |
| [Retouch](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/iop/retouch.c#L67) | Pinned source | Clone/heal/blur/fill, masks and wavelet scales. |
| [Healing solver](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/common/heal.c#L30) | Pinned source | Difference field, Laplace boundary-value solve and iterative relaxation. |
| [Command-line rendering](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/cli/main.c#L75) | Pinned source | Documented arguments and isolated library/sidecar defaults. |
| [Lua GUI actions](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/lua/gui.c#L109) | Pinned source | Calls shared action dispatcher; not proof of full headless parameter coverage. |
| [Lua image operations](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/lua/image.c#L96) | Pinned source | Sidecar/style/history/duplicate bindings. |
| [D-Bus interface](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/common/dbus.c#L30) | Pinned source | Open/Quit/Lua with conditional Lua support. |
| [Lightroom sidecar translation](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/develop/lightroom.c#L50) | Pinned source | Selected fields mapped to darktable modules; not Adobe rendering. |
| [Export implementation](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/imageio/imageio.c#L991) | Pinned source | Shared pixelpipe with output dimensions, color and format handling. |
| [Sidecar workflow](https://docs.darktable.org/usermanual/5.6/en/overview/sidecar-files/sidecar/) | Official 5.6 manual | Read-only inputs, duplicate naming and database precedence; configurable writes. |
| [Pixelpipe and order](https://docs.darktable.org/usermanual/5.6/en/darkroom/pixelpipe/the-pixelpipe-and-module-order/) | Official 5.6 manual | User-visible order, fast/HQ modes; default-filmic prose is stale against source. |
| [History-stack behavior](https://docs.darktable.org/usermanual/5.6/en/module-reference/utility-modules/darkroom/history-stack/) | Official 5.6 manual | History navigation/compression and distinction from processing order. |
| [Memory and performance](https://docs.darktable.org/usermanual/5.6/en/special-topics/mem-performance/) | Official 5.6 manual | Resource model and tuning context; not a benchmark. |
| [AgX controls](https://docs.darktable.org/usermanual/5.6/en/module-reference/processing-modules/agx/) | Official 5.6 manual | Tone/primaries/look controls; complements actual implementation. |
| [Bayer AI preparation](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/common/ai/restore_raw_bayer.c#L291) | Pinned source | CFA packing, model-output gain handling and reconstruction. |
| [Linear RAW AI preparation](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/common/ai/restore_raw_linear.c#L246) | Pinned source | Minimal sensor pipeline, model color transforms and scale normalization. |
| [RGB AI preparation](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/common/ai/restore_rgb.c#L79) | Pinned source | RGB transfer functions, tiles and output reconstruction. |
| [AI mask finalization](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/src/develop/masks/object.c#L1093) | Pinned source | Vectorizes generated mask into ordinary path groups; separate optional raster PNG export. |
| [Resource-level configuration](https://github.com/darktable-org/darktable/blob/03179f8e080aa9cedebfe14b098b7ba88940a292/data/darktableconfig.xml.in#L285) | Pinned source | Small/default/large resource policy and explanatory budgets. |

The register contains **81 entries**, including **65 distinct pinned repository files** and **5 official manual pages**. Release/tag and submodule-tree links supplement those entries. Counts describe research coverage, not independent confirmation of each claim.
