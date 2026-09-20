# Scaffold verification status

S0 is accepted on native M4 evidence and the recorded portable build/package run. Fresh hosted verification of the closure snapshot remains unfinished. Manual Windows/Linux desktop checks and manual license/native/asset reviews are deferred, not passed.

## Verified hosted baseline

[Run 35465688171](https://github.com/Willyham/lightwell/actions/runs/35465688171), commit `c0ad340`, passes the dependency, fixture, macOS, Windows and Ubuntu jobs. Ubuntu includes empty/load/replacement renderer smoke under Xvfb with llvmpipe Vulkan.

The dependency job applies the [reviewed expiring policy](advisory-policy.md) to two exact maintenance-advisory versions. Passing this policy does not mean those dependencies have been removed or the complete manual notice audit is done.

Hosted compilation is not native desktop acceptance: Windows build runners do not prove Windows 11 user-session behavior, and Xvfb/software Vulkan is not Linux native GPU-performance evidence. CI artifacts have seven-day retention; availability must be checked before relying on downloaded evidence.

## Latest locally verified closure snapshot

Snapshot `6b13871` includes the hardened viewer, per-stage JPEG diagnostics, optimized development launch, package build identities, hosted runtime-import collection and eight Linux renderer scenarios against the packaged executable. No hosted result is recorded for this snapshot. Source push requires explicit authorization.

Local checks, Doctor and host packaging pass. A separate clean local clone of that snapshot also passes with a warm shared Cargo dependency/build cache; this is not a clean-cache benchmark. Packaged failed-replacement smoke passes from that checkout with fresh output and checkout-owned fixtures.

Tested macOS package: `artifacts/s0-closure-package/lightwell-development.zip`.

| Identity | SHA-256 |
| --- | --- |
| Package | `65298bd8cb3a3c6acb7fdb45845bc36555cbd9dbcd8f7378b5ca68fb6c1cf9ce` |
| Binary | `505bfd3cdf7a848583420afee1e1fa84769116d6e53da05a09d774df62fccd74` |
| Lockfile at package assembly | `918f580c8443d810f7e5c34ff73829288dc5e1acf173c40eb5b2a03daeded67e` |

The package records a dirty pre-commit revision; the hashes identify the tested artifact. `otool -L` reports system frameworks/libraries only, with no Homebrew/developer-library path. This does not prove minimum-macOS execution.

`artifacts/s0-closure-load`, `artifacts/s0-closure-replacement` and `artifacts/s0-closure-large60` pass native M4 Metal smoke with that packaged executable. The replacement frame was visually inspected: the oriented image remains visible with a legible error and no clipping. Source hashes remain unchanged. Native keyboard/picker/resize evidence and owner-observed manual JPEG opening are documented in [hardening results](s0-hardening-results.md).

## Outstanding work

Run and inspect the current hosted workflow and Windows/Linux packages, including runtime imports and expanded packaged Linux smoke. Complete the cross-platform verification of the [bootstrap playbook](bootstrap-playbook.md). Keep these gaps separate from the accepted S0 milestone and the paused editor implementation.
