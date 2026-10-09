# Version 0.1.1 verification — 2026-10-09

Local Windows x64 checks used the isolated GNU toolchain and the full pinned
native engine with Vulkan and CPU fallback. Private recordings, transcripts and
diagnostic logs remain outside Git.

## Automated and package checks

- `scripts/check.ps1`: format, clippy with warnings denied, 15 ordinary tests
  and release compilation. Two desktop/model tests remain intentionally ignored
  in ordinary checks; ignoring them is not a pass.
- Explicit CPU `native_smoke`: cancellation, session recovery and model reload pass.
- `scripts/smoke-installer.ps1`: separate QA installation, upgrade, full notices,
  absence of bundled GGUF weights, uninstall and preservation of unowned files pass.
- Standalone setup imports only Windows system DLLs; it needs no adjacent native
  inference DLL or script runtime to start.
- The new setup is approximately 55.2 MiB, versus 758.1 MiB for 0.1.0. The reduction
  comes from excluding weights; the native application/runtime still ship together.
- A real native HTTPS download of Moonshine Tiny verifies size and hash. Cancelling
  a download leaves no `.part` file or incomplete model exposed to inference.

## Recognition checks

The 48.17-second existing private regression WAV uses the same input as the saved
baseline. Full source compilation retains the recommended Parakeet result exactly.
Alternative checks establish nonempty output, not a universal accuracy rating.

| Model | Backend | Load | Recognition | Result |
| --- | --- | --- | --- | --- |
| Parakeet TDT v3 Q8_0 | Vulkan | 625 ms | 3,012 ms | Exact saved baseline |
| Nemotron Streaming 3.5 | Vulkan | 615 ms | 2,133 ms | Nonempty multilingual output |
| Qwen3-ASR 0.6B | Vulkan | 1,007 ms | 12,526 ms | Nonempty multilingual output |
| Whisper Medium | Vulkan | 722 ms | 16,833 ms | Nonempty multilingual output |
| Moonshine Tiny, separate English fixture | Vulkan | 135 ms | 1,224 ms | Nonempty output |

The recommendations list contains the four tested multilingual choices. Canary
180M returned empty output for the Spanish regression fixture, also observed in
the existing Handy baseline for that fixture; it remains in the full catalog,
but is not in the initial recommendations. Its English Handy baseline is nonempty.
The remaining catalog entries have compiler/metadata coverage, not exhaustive
recognition validation. Large models require suitable memory and may have limits
or model-specific behavior; the catalog snapshot alone does not prove accuracy.

These are file recognition measurements, not live shortcut-to-editor latency.

## Physical interface checks

- The actual shortcut dialog accepts and saves Ctrl+Space; stored modifiers are
  Ctrl alone. F keys and modifier validity are covered by ordinary tests.
- In a separate test editor, releasing the first shortcut leaves the application
  listening. The second press transitions to recognition. Cancel/exit invalidates
  the session and does not store microphone PCM.
- Setup choices and model selection use the same window handle. Recommended and
  full-catalog views switch in place. Setup was physically centered on the active
  monitor; labels and surrounding areas use one system-color background.
- Setup copy contains no application-license paragraph or upstream product naming.
  Spanish follows the current Windows display language. Language-ID unit checks
  cover Spanish regional IDs and English fallback; an English Windows desktop
  was not physically tested.
- README images are actual packaged/native window captures, including the
  corrected shortcut and wizard pages. The earlier pill image remains explicitly
  labeled as a real inspection-mode capture.

Live speech accuracy through the full toggle-to-paste path, browser insertion,
physical multi-monitor/dynamic DPI, and dark/high-contrast theme combinations
still require dedicated manual QA. Previous 0.1.0 clipboard/paste and resource
measurements remain historical evidence rather than new measurements here.
