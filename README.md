# Lightwell

An open-source, non-destructive desktop photo editor for macOS, Windows and Linux, built around a small, fast core that people, programs and agents can operate equally. Project code is [GPL-3.0-or-later](LICENSE).

**Status:** S0 is accepted and its separate viewer has been retired into the editor; M1 persistent history, M2 exact transforms and M3 tool modules are implemented and verified on the owner's M4 Mac; M4 crop is next. Everything is v0. See [feature status](docs/features.md).

## Quick start

Install the pinned Rust toolchain and the platform prerequisites in [development](docs/engineering/development.md), then:

```sh
rustup toolchain install 1.94.0 --profile minimal --component rustfmt --component clippy
cargo xtask check
cargo xtask develop --open fixtures/s0/orientation-6.jpg
```

## Documentation

- [AGENTS.md](AGENTS.md): pillars, workflow and engineering rules. Read this first, human or agent.
- Product: [roadmap](docs/plan.md), [decisions](docs/decisions.md), [feature status](docs/features.md), [user guide](docs/user-guide.md).
- Design: [architecture](docs/design/architecture.md), [milestone contracts](docs/design/history-first-roadmap.md), [versions and lineage](docs/design/versions-and-lineage.md), [tool modules](docs/design/modules-and-api.md).
- Specs: [layers, history and transforms](docs/specs/edit-history.md), [crop, export and conflicts](docs/specs/single-image.md), [source recovery](docs/specs/source-recovery.md), [performance](docs/specs/performance.md).
- Engineering: [development and verification](docs/engineering/development.md), [performance rules](docs/engineering/performance-rules.md), [platforms](docs/engineering/platforms.md), [dependencies](docs/engineering/dependencies.md).
- Research: [stack and library options](docs/research/technical-options.md), [Lightroom Classic](docs/research/lightroom/README.md), [darktable](docs/research/darktable/README.md).
- [Task plans](tasks/README.md).

Third-party components keep their own licenses. The manual license, native and asset review is deferred and incomplete.
