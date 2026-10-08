# Native integration checkpoint

Windows x64 / Ryzen 5 5500X3D / RX 580, local GNU release with pinned Vulkan engine.
This is an intermediate functional checkpoint, not final product acceptance.

- Real Windows Notepad: private 48.168-second WAV recognized and pasted into a
  new, separate document. Saved UTF-8 text (574 characters) is exactly equal to
  the previous Parakeet baseline. No recording/transcript was uploaded.
- Release-equivalent file submission to post-injection clipboard read: 3522 ms.
  This receipt is a latency proxy. Actual insertion was independently observed
  and checked from the saved document, without claiming exact insertion timing.
- Five regular tests passed, including invalid audio, cancelled/paused session
  gates and 48/44.1 kHz tail preservation. Native model smoke remains explicit.
- Actual default microphone probe: 48 kHz, 4.98 seconds, 79680 resampled samples,
  RMS 0.00000972, peak 0.000862, observer voice score 0.3145. This proves capture
  and conversion of quiet room input; it is not proof of live spoken dictation.
- Explicit local Windows clipboard test passed: delayed-render receipt restores
  Unicode, HTML and bitmap pixels; a newer copy is preserved; cancellation before
  insertion restores the original. This test caught and fixed a close-time render
  that could overwrite restored Unicode. It pumps actual Win32 messages and is
  ignored by ordinary CI to avoid touching a shared desktop clipboard.
- No web runtime is part of the build. Microphone PCM stays in memory. Final pill,
  live-speech dictation, browser/rich clipboard, DPI and installer acceptance are
  pending and must be recorded separately before release.

Private evidence is kept in ignored local/. Existing Notepad tabs were preserved.
