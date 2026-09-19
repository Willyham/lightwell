# Initial hosted CI results

Verification date: 2026-09-19. S0 acceptance remains incomplete.

The expanded workflow at `16a2003` is [run 35460353189](https://github.com/Willyham/lightwell/actions/runs/35460353189). Its results:

- macOS: repository checks, formatting, Clippy, six core tests, optimized build and development package passed.
- Fixtures on Ubuntu: all 16 hashes, content checks and byte-for-byte regeneration passed.
- Windows: checks, optimized build and development package passed after the encoding fix.
- Ubuntu: checks, optimized build and packaging passed. Headless smoke failed at startup because libxkbcommon-x11.so was missing. Retained subprocess evidence identified the runtime prerequisite; the follow-up installs libxkbcommon-x11-0.
- Dependency policy: license/source checks passed. Blocking advisory check failed on exactly the documented maintenance findings for paste and ttf-parser. No ignore was introduced.

Packages and synthetic renderer evidence are retained as run artifacts for seven days. Hosted Windows builds are not Windows 11 desktop acceptance. Any Xvfb/software-renderer result is headless functional evidence, not native Linux desktop/GPU validation. Full asset/native license review and S0's remaining behavior/performance acceptance remain open.

Follow-up run for Linux runtime correction: [35460833672](https://github.com/Willyham/lightwell/actions/runs/35460833672), commit `18dc4ad`. Fixture checks and macOS/Ubuntu build jobs pass. All three Linux smoke scenarios pass on Xvfb with llvmpipe (LLVM 20.1.2, 256 bits), Vulkan. The retained failed-replacement screenshot was visually reviewed and matches the state/pixel checks. Linux binary SHA-256 is `f1a90fcba5f0dca511d0e756dcc68de78aa522712c1be90e3dad5e3495f62e1f`. Windows final rebuild also passed. The run is complete: all platform build/package jobs and fixture checks pass; only the dependency advisory job fails.

The documentation-only evidence follow-up uses `[skip ci]` after local plan/link validation; application and workflow verification refers to the completed `18dc4ad` run above. No code or workflow changed in that follow-up.

## Green run after advisory review

[Run 35465688171](https://github.com/Willyham/lightwell/actions/runs/35465688171), commit `c0ad340`, passes every job: dependencies, fixtures, macOS, Windows and Ubuntu. The Ubuntu job includes empty/load/replacement headless renderer smoke. The dependency gate now applies the [reviewed expiring policy](advisory-policy.md); this accepts only two maintenance IDs for pinned versions and does not mean the dependencies were removed. Six policy regression tests and live UTC/version/task validation run in the shared check command. No new vulnerability or license finding is suppressed by these exceptions.

## Current scaffold closure verification

Local source snapshot `6b13871` on `codex/s0-closure` includes the hardened viewer, per-stage JPEG diagnostics, optimized normal development launch, package build identities, hosted runtime-import collection and eight Linux renderer scenarios against the packaged executable. **No hosted result exists for this snapshot yet.** Automatic approval review rejected pushing to the existing GitHub remote pending explicit owner authorization; source export has not occurred.

Local `cargo xtask check` passes, as do Doctor and host packaging. A separate clean local clone of `6b13871` also passes all checks (shared, warm Cargo dependency/build cache; not a clean-cache benchmark). The documented packaged failed-replacement scenario passes from that checkout using fresh output and checkout-owned fixtures.

Mac package: `artifacts/s0-closure-package/lightwell-development.zip`, SHA-256 `65298bd8cb3a3c6acb7fdb45845bc36555cbd9dbcd8f7378b5ca68fb6c1cf9ce`. Binary SHA-256 `505bfd3cdf7a848583420afee1e1fa84769116d6e53da05a09d774df62fccd74`; lock SHA-256 `918f580c8443d810f7e5c34ff73829288dc5e1acf173c40eb5b2a03daeded67e`. The package was assembled before the commit and truthfully records a dirty prior revision; these hashes identify the tested artifact. `otool -L` reports system frameworks/libraries only, without a Homebrew/developer-library path. This does not prove minimum-macOS execution.

`artifacts/s0-closure-load`, `artifacts/s0-closure-replacement` and `artifacts/s0-closure-large60` pass native M4 Metal smoke with that actual packaged executable. The failed-replacement frame was visually inspected: the last oriented image remains visible with a legible error and no clipping. Source hashes remain unchanged. Prior native keyboard/picker/resize evidence and the owner's successful manual JPEG open remain recorded separately in the hardening report. The [bootstrap playbook](bootstrap-playbook.md) now supplies fresh-checkout instructions.

TASK-052/053/055/056 still need current hosted results and package/runtime-artifact inspection. TASK-062's local playbook verification is done, but its Windows/Linux package prerequisites remain outstanding. TASK-063 and the M1 gate TASK-064 remain pending. Manual Windows/Linux desktop checks and manual license review remain deferred; no claim of M1 implementation is made.
