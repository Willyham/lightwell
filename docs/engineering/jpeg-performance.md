# Real JPEG loading investigation

Status: optimized development launch and stage diagnostics implemented; local measurements recorded 2026-09-19.

The owner reports roughly ten seconds to open a local 10 MB JPEG. Reproduce using the supplied private `fixtures/jpg/will-sapa-drone.jpg` (3389 × 4236, 10,444,237 bytes) without modifying or committing it. Compare debug and optimized native Metal builds with actual renderer captures. Measure file reading, header/profile validation, pixel decoding, orientation, preview resizing, RGBA conversion and GPU upload separately.

Initial reproduction: debug image preparation 10,219.7 ms and request-to-capture 10,273.1 ms; existing optimized package preparation 316.5 ms and capture 351.9 ms. Upload 19.5/16.1 ms respectively. These are individual warm-filesystem observations, not statistical performance gates.

Scope: expose stage timing in development diagnostics, make `cargo xtask develop` use the optimized release profile by default, retain an explicit debug launch, and document the distinction. Preserve the existing Triangle preview filter, orientation/color behavior, worker scheduling and source bytes. No decoder replacement, image-quality shortcut, editing features or dependency changes are planned.

Acceptance: identify the dominant debug stage; reproduce optimized loading with the normal launcher; inspect actual image content; verify source hash unchanged; pass repository checks and validate every active task graph. Record measurements and limitations here. The original performance baseline covered only optimized synthetic images, so it failed to reveal the slow default development experience.

## Measured result

Native Apple M4 Pro / Metal, macOS, 48 GiB RAM. Two app launches per profile (one before and one after instrumentation), uncontrolled warm filesystem cache, 1920 × 1280 renderer captures. These results explain this reproduction; they are not cold-storage measurements or percentile claims. Native picker selection time is excluded. Compilation is excluded.

| Stage | Unoptimized debug (ms) | Optimized release (ms) |
| --- | ---: | ---: |
| Read source | 3.27 | 1.43 |
| Header, profile and orientation metadata validation | 2.39 | 1.08 |
| JPEG pixel decode | 2396.18 | 101.23 |
| Apply orientation | <0.01 | 0.02 |
| Triangle preview resize | 6036.60 | 200.84 |
| Convert preview to RGBA | 1838.57 | 6.74 |
| Total worker preparation | 10277.01 | 311.34 |
| GPU allocation/upload | 15.76 | 16.16 |
| Open request to renderer capture | 10334.33 | 348.70 |

The 4236-pixel source height crosses the 4096-pixel limit and triggers resampling to 3277 × 4096. Unoptimized CPU loops dominate the ten-second delay. Disk reading and GPU upload do not explain it. No image algorithm or filter was changed: the fix is selecting release optimization for the normal development launcher. Explicit `develop --debug` and plain `cargo run` still produce slow unoptimized behavior. Release compilation may take longer on the first launch.

Evidence directories (ignored local artifacts): `artifacts/jpeg-investigation-debug`, `artifacts/jpeg-investigation-release`, `artifacts/jpeg-stages-debug`, `artifacts/jpeg-stages-release`. The stage runs exercised the actual `cargo xtask develop --debug` and default `cargo xtask develop` dispatch. The final release frame was visually inspected: terraced fields and the winding road fill the correctly proportioned portrait Fit image. The displayed image rectangle was pixel-identical between the instrumented debug and release renderer captures. This is output equivalence, not an independent color-accuracy certification.

Source SHA-256 before/after: `8706b1fe34fc0e6d62d483a6868a5764e97429ca32a8dd9147dd66501a2e93b5`. The JPEG remains local and is ignored by Git. Repository check passed: task graphs and Markdown links, runner regressions, formatting, Clippy, core/application tests and doc tests. No manual license or Windows/Linux desktop checks were added.

Reproduce with a fresh evidence directory each time:

```sh
cargo xtask develop --open fixtures/jpg/will-sapa-drone.jpg --evidence-dir artifacts/jpeg-new-release
cargo xtask develop --debug --open fixtures/jpg/will-sapa-drone.jpg --evidence-dir artifacts/jpeg-new-debug
```

Both profiles are distinguishable in startup diagnostics through `debug_assertions`; all six worker stages are logged on each decoded image. This flag describes assertion settings rather than guaranteeing arbitrary custom-profile optimization. The measured optimized full-image resize still costs about 201 ms; viewport-sized previews and reduced-resolution decoding are future performance opportunities requiring separate correctness and zoom requirements, not changes made here.

Measured instrumented binary SHA-256:

- `debug`: `6a483e233ae0a9ae0416f5e351613d2f8e6b2b11e2cb0cb3f91d5f0c6add84c6`
- `release`: `505bfd3cdf7a848583420afee1e1fa84769116d6e53da05a09d774df62fccd74`
