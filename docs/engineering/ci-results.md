# Initial hosted CI results

Verification date: 2026-09-19. S0 acceptance remains incomplete.

The first pushed scaffold commit, `05937b7`, triggered [run 35460222072](https://github.com/Willyham/lightwell/actions/runs/35460222072). macOS and Ubuntu passed their build/package jobs. Windows failed because Python used the system code page to read UTF-8 Markdown. The fix makes text-file encoding explicit and sets UTF-8 mode for the Python command runner.

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
