# Verification

## Current coverage

Bootstrap CI checks Rust format, clippy, tests and a Windows release library
build. There are no behavior tests yet because the bootstrap only establishes
the package. No inference, microphone, tray, paste or installer test is claimed.

## Acceptance work

Session tests must reject results after Escape, pause and close. Cleanup fixtures
must preserve bilingual meaning, names, numbers, dates, negatives, English terms
and code. Model checks must verify the pinned size/hash before packaging. Local
private recordings never enter CI, PR bodies or published artifacts.

Real Windows checks include microphone capture, editor/browser insertion,
clipboard recovery, focus, hotkey release, errors, pause/close, multiple DPI and
available monitors. Capture evidence from the real executable. A banner or mockup
is not application evidence. Record checks that cannot be automated as pending.

Measure resident and private memory separately, CPU idle, process descendants,
VRAM, cold/warm model load and release-to-insertion. Compare repeated runs on the
same machine/model/audio with the previous accepted baseline. Full-WAV inference
time is not release-to-insertion latency. Packaging acceptance requires a clean
offline install/uninstall without Handy, Node, Python or WebView2 dependencies.
