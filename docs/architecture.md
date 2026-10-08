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

The planned overlay is a 112 by 34 logical-pixel pill with a luminous dot and
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
the pinned payload; normal loading checks size and architecture. Capture,
resampling/VAD, conservative cleanup and Windows UI/insertion remain to be added.

Inherited web/audio-cloud/history tooling has been removed from the working tree;
upstream references remain in Git history. GPU modules are dynamically loaded so
absence of Vulkan can fall back to CPU. No CUDA is required. Pending packaging
must stage the native DLLs and the applicable compiler runtimes/license texts.
