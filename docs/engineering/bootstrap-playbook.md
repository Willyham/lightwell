# Bootstrap playbook

This is the maintained S0 load-only viewer: one Open action and automatic Fit. There is no catalog, editing, export or MCP yet. Supported input is 8-bit RGB/greyscale JPEG, EXIF 1–8, untagged-as-sRGB or supported standard sRGB profiles. Other gamuts/CMYK are rejected. Sources are limited to 128 MiB encoded, 64 megapixels and 16384 pixels per dimension; uploads are capped at 4096 pixels on the longest side. Originals are read-only.

## Set up a checkout

Install Git and Rust through rustup. Use the [platform prerequisites](platforms.md): macOS needs Xcode Command Line Tools; Windows needs MSVC C++ Build Tools and Windows SDK; Ubuntu 24.04 needs `build-essential pkg-config libx11-dev libxkbcommon-dev libwayland-dev libvulkan-dev`, plus runtime `libxkbcommon-x11-0`, a graphics driver and a working desktop portal for the picker. No command below silently installs system tools.

From a fresh checkout:

```sh
rustup toolchain install 1.94.0 --profile minimal --component rustfmt --component clippy
cargo xtask doctor
cargo xtask check
cargo xtask build --release
cargo xtask develop --open fixtures/s0/orientation-6.jpg
```

Doctor does not prove an available desktop/GPU. `develop` uses release optimization; `develop --debug` explicitly selects the slower debugger build. The initial Cargo dependency fetch and compilation require network/time.

## Capture and inspect a real render

Use the same Rust runner on every platform:

```sh
cargo xtask smoke --scenario load --output artifacts/playbook-load
cargo xtask smoke --scenario replacement --output artifacts/playbook-replacement
```

Use new output directory names on every run. The release binary must already exist. A native graphical session is required; unsupported display access is not a pass. The replacement scenario loads a valid photo then malformed input and checks that the previous photo remains visible.

Inspect `artifacts/playbook-load/result.json` for `status: passed`, binary/lock/source hashes and platform identity; read `app/events.jsonl`, `app/state.json` and `app/result.json`. Open the `app/frame-*.png` files and correlate each frame's generation and dimensions with the recorded state. These PNGs are actual renderer readbacks, not OS desktop screenshots. Blank/stale frames fail pixel checks. `reproduce.md` and `subprocess.log` preserve exact arguments and failure output. A failed run returns nonzero; absence of a PNG is not success. Evidence is bounded by application/process deadlines.

Other scenarios are `empty`, `invalid`, `repeated`, `alternating`, `large24`, `large60`. Before the large cases run `cargo xtask generate-fixtures` (requires a new `fixtures/generated` directory; preserve earlier benchmark inputs elsewhere before replacing disposable generated workloads). Only synthetic fixtures belong in hosted artifacts. Keep personal photos in ignored `fixtures/jpg/` or `private/`; never force-add originals or local diagnostics.

## Package and verify the actual executable

```sh
cargo xtask package --output artifacts/playbook-package
```

The output contains an unsigned ZIP on macOS/Windows or `.tar.gz` on Linux, `checksums.txt`, and a `Lightwell` directory with notices and `build.json`. The build record identifies source revision, dirty working-tree status, target, profile and binary/lock hashes. A dirty revision alone cannot identify source changes. This is repeatable packaging, not a claim of byte-identical reproducible binaries.

Run smoke against the packaged executable using `--binary`:

```sh
cargo xtask smoke --binary artifacts/playbook-package/Lightwell/Lightwell.app/Contents/MacOS/lightwell --scenario replacement --output artifacts/playbook-packaged
```

Linux uses `artifacts/playbook-package/Lightwell/lightwell`; Windows uses `artifacts/playbook-package/Lightwell/lightwell.exe`. After unpacking, launch the same executable without evidence arguments for ordinary use. macOS bundles are unsigned and not notarized; use the normal system approval flow if downloaded software is quarantined. No toolchain is needed to run the package, but platform runtime/graphics prerequisites still apply. Do not disable OS security globally.

## Evidence and limitations

The hosted workflow uses clean checkouts with locked checks/build/package on macOS arm64, Windows x64 and Ubuntu x64. Linux captures run under Xvfb with software Vulkan; this proves functional rendering, not native performance or Wayland/picker behavior. Runtime imports are retained alongside hosted Linux/Windows packages. Manual Windows/Linux desktop and manual license reviews are deferred by the owner. Existing automatic dependency policy remains enabled.

[CI results](ci-results.md), [native hardening results](s0-hardening-results.md) and [real JPEG performance](jpeg-performance.md) identify verified runs and limitations. The owner confirmed manual JPEG selection on M4; automated picker selection is unreliable. Native accessibility, minimum-version execution, broad color-management and long-run memory plateau are not established. Never infer those results from a green compile or screenshot.
