# Initial hosted CI results

Verification date: 2026-09-19. S0 acceptance remains incomplete.

The first pushed scaffold commit, `05937b7`, triggered [run 35460222072](https://github.com/Willyham/lightwell/actions/runs/35460222072). macOS and Ubuntu passed their build/package jobs. Windows failed because Python used the system code page to read UTF-8 Markdown. The fix makes text-file encoding explicit and sets UTF-8 mode for the Python command runner.

The expanded workflow at `16a2003` is [run 35460353189](https://github.com/Willyham/lightwell/actions/runs/35460353189). Its verified results so far:

- macOS: repository checks, formatting, Clippy, six core tests, optimized build and development package passed.
- Fixtures on Ubuntu: all 16 hashes, content checks and byte-for-byte regeneration passed.
- Windows: checks passed after the encoding fix; optimized build is pending completion.
- Ubuntu: checks, optimized build and packaging passed. Headless smoke failed at startup because libxkbcommon-x11.so was missing. Retained subprocess evidence identified the runtime prerequisite; the follow-up installs libxkbcommon-x11-0.
- Dependency policy: license/source checks passed. Blocking advisory check failed on exactly the documented maintenance findings for paste and ttf-parser. No ignore was introduced.

Packages and synthetic renderer evidence are retained as run artifacts for seven days. Hosted Windows builds are not Windows 11 desktop acceptance. Any Xvfb/software-renderer result is headless functional evidence, not native Linux desktop/GPU validation. Full asset/native license review and S0's remaining behavior/performance acceptance remain open.
