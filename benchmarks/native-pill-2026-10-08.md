# Native UI checkpoint

Windows x64, Ryzen 5 5500X3D, RX 580; local GNU release. Compare with the
[core checkpoint](core-2026-10-08.md) and [native integration](windows-integration-2026-10-08.md).
This is actual executable evidence; the offline installer is a later milestone.

| Check | Observed result |
| --- | --- |
| Normal 100% scale | 96 DPI, physical frame 128 × 50, body 112 × 34 logical pixels |
| Actual Windows 150% scale | 144 DPI, physical frame 192 × 75; original 100% scale restored |
| Normal show focus | Preserved, measured before/after the no-activation show |
| Escape during physical capture | Capture and pill stop; private memory 22.6 MB → 5.8 MB |
| Native errors | Accessible Copy/Retry/Close buttons; retained text can be recovered |
| Tray/shortcut | Real native menu; small Ctrl+Alt+Space dialog opened and cancelled |
| Modal exit | Sent control reaches open menus; exit releases the instance |
| Private 48.168 s WAV into Notepad | Saved 574 characters exactly equal the prior Parakeet baseline |
| File fixture load / inference | 600 ms / 2811 ms; no claim of a controlled cold-cache speedup |
| Fixture trigger → clipboard read | 3501 ms receipt proxy; actual insertion independently checked |
| Deliberate target focus switch | Editor stays empty; recognition is retained for recovery |
| Explicit retry of retained text | Actual editor receives one copy; 101 ms read proxy, no new ASR or microphone |

Idle after the model has unloaded: 59.20 MiB resident, 34.74 MiB private,
5.73 MiB dedicated GPU allocation and 9.34 MiB shared GPU allocation. Over a
47.618-second measured interval the process CPU counter remains exactly 4.09375 s.
GPU counters represent process allocations, not guaranteed exclusive physical VRAM.
No working-set trimming is used. This trace starts after unloading and does not
measure the app's warm peak; the core trace separately records model residency.

Compared with the earlier unloaded core, native UI adds about 13.2 MiB resident
and 3.5 MiB private after model use. This pays for actual Windows drawing, controls
and accessibility, while idle CPU remains unchanged. Recognition text is preserved.

Isolated appearance captures use the explicit inspection shell flag because the
desktop helper excludes normal nonactivating tool windows; rendering, input and
state flow are the real app. Normal focus/resource checks do not use that flag.
Geometry tests cover 200% DPI and negative-coordinate work areas; the available
1920 × 1080 monitor exposes 100/125/150/175% settings. Physical multi-monitor,
reduced-motion/high-contrast theme changes and 200% OS scaling are not claimed.
Live spoken dictation is explicitly deferred by the user; quiet microphone capture
and private file recognition are distinct checks. Private evidence remains in local/.
