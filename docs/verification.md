# Verification

The current changes are recorded in [0.1.1 verification](verification-0.1.1.md).
The checkpoints below describe the original 0.1.0 baseline.

## Current coverage

CI checks Rust format, clippy, invalid-audio/session-invalidation tests and Windows
release builds with CPU and Vulkan. The separate CPU smoke step obtains and
verifies the pinned weights. `native_smoke` is ignored by regular cargo tests;
that ignored result does not prove inference. With weights
it checks in-flight abort without returning partial text, recovery on the same
session and reload after model/session teardown. Synthetic silence is generated
in memory; this smoke does not establish speech accuracy.

Local recognition regression compares the existing private WAV to its saved
Parakeet output, keeping text and recordings in ignored local storage. Numeric
results and current limitations belong in benchmarks/. Actual native checks now
cover quiet physical capture, Escape cancellation, saved Notepad insertion,
foreground-switch suppression, recovery paste, rich/bitmap clipboard restoration,
native tray/shortcut controls and physical 100/150% DPI. See the
[native UI checkpoint](../benchmarks/native-pill-2026-10-08.md).
Live spoken dictation is explicitly deferred by the user. Complete installed-app
residency and offline install/upgrade/uninstall/dependency acceptance are recorded
in the [installed checkpoint](../benchmarks/installed-app-2026-10-08.md). Browser
insertion remains pending: automatic browser policy rejected the local-file test.
Physical multimonitor/200% DPI and high-contrast/reduced-motion theme tests are
also unperformed; do not describe the geometry unit tests as physical coverage.

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

Cleanup regression: unit cases protect names/dates/negation/English/code and
exercise mechanical edits and the narrow acoustic/repetition rules. Use explicit
WAV --cleanup runs for local bilingual baseline comparisons; never publish text.
