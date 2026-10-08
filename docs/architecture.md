# Native architecture

The root Cargo build has no web runtime. `native/engine/ffi.rs` uniquely owns the
native model/session; `worker.rs` serializes recognition, bounds queued audio to
one job, and releases the model after 30 seconds idle. With no model it blocks on
incoming commands without periodic wakeups. `session.rs` is the UI-side generation
gate: only an active session's result may be consumed once. Cancellation flags
also reach the native callback; partial text is never returned as success.

The UI thread owns Windows resources; a worker owns inference. Session IDs must
invalidate late completions after cancellation. Audio stays in memory and capture
opens only during a session. Recognition consumes mono f32 at 16 kHz. A single
model will unload after an initial 30-second idle timeout, subject to measurement.

The overlay is a 112 by 34 logical-pixel pill with a luminous dot and
real microphone levels. Hidden means no animation timer or drawing. It must
respect focus, per-monitor DPI, high contrast and reduced motion. Win32 owns the
tray and small settings dialogs. Error text uses DirectWrite.

`scripts/prepare-parakeet.ps1` reduces the pinned transcribe.cpp source to Parakeet
and shared ggml/frontend code. It excludes other family sources and registry
entries, the legacy Whisper loader, miniz and embedded diarization. This offline
v3 product rejects multitalker bundles. Numerical inference for the selected
model is unchanged. The small Rust FFI owner adapts upstream MIT ownership and
abort patterns without linking multi-family safe wrappers.

`native/audio.rs` accepts mono 16 kHz PCM16/float32 WAV for regression;
`native/main.rs` reports cold/warm native load/run timing. SHA-256 validation is
explicitly separate from measured native load. The future installer verifies
the pinned payload; normal loading checks size and architecture.

`native/audio/capture.rs` opens CPAL only while recording. Its callback mixes mono
into a two-second preallocated ring and publishes nine real RMS segments; the
collector appends bounded PCM outside the audio callback. `resample.rs` drains
Rubato's delay line so the final syllable is retained. Earshot observes frames
without removing audio; its score is diagnostic, never a speech gate.

`native/platform/windows/runtime.rs` owns the single-instance message loop,
foreground notifications and tray command routing. The keyboard hook is event
driven; no idle keyboard-poll timer. `application.rs` coordinates the session
gate, capture, worker and paste. Settings are independent in LocalAppData/DictadoLite.
`clipboard_snapshot.rs` copies supported global-memory, bitmap and enhanced
metafile formats before any clipboard change; unsupported handles fail closed.
`clipboard.rs` owns a separate delayed-render message thread, waits for a
post-injection read and restores only the still-owned clipboard sequence. A new
user copy is never overwritten. Paste requires unchanged destination focus and
released modifiers. Failed results remain in memory for explicit recovery copy.
`overlay/` implements work-area/DPI geometry, a Direct2D premultiplied surface and
the nonactivating window/state bridge. Normal microphone bars read actual RMS;
errors draw DirectWrite text with native accessible buttons. The surface is freed
when hidden. Clipboard publication waits for closed menus and released shortcut
modifiers; normal drawing never takes destination focus.

Inherited web/audio-cloud/history tooling has been removed from the working tree;
upstream references remain in Git history. GPU modules are dynamically loaded so
absence of Vulkan can fall back to CPU. No CUDA is required. Pending packaging
must stage the native DLLs and the applicable compiler runtimes/license texts.
