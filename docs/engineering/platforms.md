# S0 platform and verification matrix

Status: **accepted target scope, provisional engineering baselines**. The owner selected macOS Apple Silicon, Windows x64 and Linux x64 with unsigned development packages, then delegated initial OS/runtime and Linux desktop choices to engineering. These are intended support/testing boundaries, not claims of completed compatibility.

| Target | Initial baseline | Artifact | Graphics/window path | Verification route |
| --- | --- | --- | --- | --- |
| `aarch64-apple-darwin` | macOS 14 or later | `.app` in an unsigned ZIP | Metal, AppKit | Available M4 Pro / macOS 26.5.2 desktop; macOS 14 floor check remains outstanding |
| `x86_64-pc-windows-msvc` | Windows 11 24H2 or later | executable and notices in ZIP | wgpu DX12, native Win32 picker | Native x64 Windows desktop required; no current session established |
| `x86_64-unknown-linux-gnu` | Ubuntu 24.04 LTS x64, glibc 2.39 build baseline | directory in `.tar.gz` | Vulkan; GNOME Wayland primary, X11 also required for S0 checks | Native x64 Linux desktop required; no current session established |

macOS 14 is a conservative project baseline above Rust's Apple Silicon minimum; it avoids promising every upstream-supported OS. Windows 11 and Ubuntu 24.04 keep the initial test matrix narrow. Baselines may be revised from measured dependency/runtime evidence, with changes recorded here. A build on a newer SDK does not prove execution on the floor. CI must compile all three targets; desktop test results must separately identify the actual OS, architecture and backend. Linux ARM64 or Windows ARM emulation may supplement, not replace, the x64 matrix.

## Native prerequisites

- All development hosts: Rust 1.94.0 with rustfmt and Clippy; Python 3.10+ for repository checks. Application dependencies remain pinned by the selected workspace and lockfile. Pillow 12.2.0 is optional fixture/capture-verification tooling.
- macOS: Xcode Command Line Tools/SDK, native linker, Metal-capable Apple Silicon and an unlocked graphical desktop for interaction verification. Development bundles are unsigned; no notarization/store signing is promised.
- Windows: Visual Studio Build Tools with the C++ toolchain and Windows SDK for the MSVC target, plus a compatible DX12 driver and desktop session. Inspect the produced artifact's runtime imports before declaring packaging self-contained.
- Linux: C/C++ build tools and pkg-config, development/runtime window and keyboard libraries for winit X11/Wayland, Vulkan loader/driver, and XDG desktop portal plus a working GNOME file chooser backend. Use the native Ubuntu 24.04 build environment to avoid accidentally raising the glibc baseline. Record actual package names in the verified setup instructions when the selected configuration builds there.

The current rfd trial uses the XDG portal rather than GTK development bindings. An absent portal is a runtime file-picker failure, not proof of a working Linux app. Software Vulkan may support functional CI; label it and do not use it as native GPU performance evidence.

## Evidence inventory and closure

Available host: M4 Pro, 14 CPU/20 GPU cores, 48 GB unified memory, macOS 26.5.2. Existing native Metal renderer-readback captures at 2× scale are documented in [probe results](s0-probe-results.md). The owner's latest answer explicitly permits locally built software to run; a subsequent UI attempt was blocked by a locked Mac, not missing owner permission. Native picker/focus/resize checks resume when the desktop is unlocked.

Windows and Linux hardware/session access, minimum-version checks, and packaging/runtime validation remain outstanding. This explicitly records missing test routes and does not prevent platform **decision** TASK-023 from completing. Those checks remain required implementation tasks TASK-055/056/058/059 and S0 gate TASK-063. Do not close S0 using compilation, a VM, or the current macOS probe alone. No VM installation, infrastructure purchase or public distribution is part of this matrix decision.

## Sources and selection basis

The [Rust Apple target documentation](https://doc.rust-lang.org/rustc/platform-support/apple-darwin.html) identifies ARM64 macOS support. The [wgpu backend documentation](https://wgpu.rs/doc/wgpu/struct.Backends.html) identifies Metal, DX12 and Vulkan platform routes. The locked `wgpu` 27.0.1, `winit` 0.30.13 and `rfd` 0.15.4 manifests/sources were inspected locally; the [rfd feature list](https://docs.rs/crate/rfd/0.15.4/features) corroborates its selectable portal path. Our narrower OS/version choices are engineering scope decisions under the owner's delegation, not upstream guarantees or measured compatibility results.

## Windows test availability

The owner confirmed on 2026-09-19 that no Windows 11 x64 machine is currently available. Native Windows launch/load acceptance remains pending; a hosted CI compilation or another operating system's smoke run cannot substitute. Continue independent local scaffold work without treating this as an unresolved product decision.

Linux X11 runtime requires `libxkbcommon-x11-0` in addition to the development libraries. Hosted renderer startup exposed the missing runtime library; `libxkbcommon-dev` alone did not supply it. Headless smoke installs Xvfb and Mesa Vulkan drivers separately from native desktop prerequisites.
