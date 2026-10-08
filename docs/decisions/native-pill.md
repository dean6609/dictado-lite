# A quiet native indicator

The normal body is 112 × 34 device-independent pixels, with eight-pixel logical
padding for a contained shadow. Graphite depth, a restrained rim and a small
pearl/cyan/lavender light use owned vector geometry. There is no web runtime,
backdrop-blur loop, image animation, model name, counter or visible normal text.

The visual direction draws on Apple's [materials guidance](https://developer.apple.com/design/human-interface-guidelines/materials),
[Dynamic Island session](https://developer.apple.com/videos/play/wwdc2023/10194/)
and [Siri activity presentation](https://www.apple.com/newsroom/2024/06/introducing-apple-intelligence-for-iphone-ipad-and-mac/).
No Apple artwork or proprietary assets are included.

Direct2D software rasterization draws a tiny premultiplied BGRA DIB which Win32
composes through UpdateLayeredWindow. This avoids creating a separate D3D device
for a very small surface. Gradient brushes are cached during the visible session;
hidden releases the surface and has no drawing/animation timer. Nine bars consume
actual callback RMS segments, with attack/release smoothing and a static silence
floor. Processing shows only a slow light pulse, never simulated audio waves.
Success fades for 220 ms. Reduced motion disables fades and decorative pulse;
high contrast uses Windows colors and a stronger outline.

Normal windows are tool windows with no activation and click-through behavior.
The target monitor's work area and effective DPI determine placement. Error text
uses DirectWrite, with three actual native buttons for accessibility and pointer
actions; the tray also exposes recovery copy and microphone controls. State names
and alert events are announced without adding visible labels to the normal pill.

The desktop capture helper excludes normal nonactivating tool windows. Explicit
developer `--inspect-overlay` changes shell eligibility/ownership to permit a
window capture, using the same live microphone, renderer, sizing and state flow.
It adds a temporary taskbar entry; normal operation has none. Appearance captures
are labeled as inspection captures, and normal focus/resource checks are separate.

Physical validation on the available monitor covers Windows scale 100% and 150%.
The OS lists at most 175% on this 1920 × 1080 display; 200% and negative-coordinate
work areas are covered by geometry tests. Original scale was restored to 100%.
Live spoken dictation was explicitly deferred by the user; quiet capture and WAV
recognition/paste are verified separately.
