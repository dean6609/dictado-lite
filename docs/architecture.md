# Native architecture

The root Cargo build is independent of Tauri. `native/lib.rs` defines the initial
audio contract. In upcoming milestones, small modules will own audio capture and
resampling, Parakeet model lifetime, session cancellation, conservative cleanup,
Windows insertion/tray/hotkeys and Direct2D/DirectWrite presentation.

The UI thread owns Windows resources; a worker owns inference. Session IDs must
invalidate late completions after cancellation. Audio stays in memory and capture
opens only during a session. Recognition consumes mono f32 at 16 kHz. A single
model will unload after an initial 30-second idle timeout, subject to measurement.

The planned overlay is a 112 by 34 logical-pixel pill with a luminous dot and
real microphone levels. Hidden means no animation timer or drawing. It must
respect focus, per-monitor DPI, high contrast and reduced motion. Win32 owns the
tray and small settings dialogs. Error text uses DirectWrite.

Legacy `src/`, `src-tauri/`, frontend tooling and assets are extraction references
only. They are not dependencies of the root package and will be removed after
the useful core is migrated. No ONNX, cloud client, model catalog, history store,
webview or neural cleaner belongs in the final product.
