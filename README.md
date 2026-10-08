<div align="center">

![Dictado Lite — local voice, native Windows](assets/dictado-banner.svg)

**Speak. Release. Keep writing.**

Local Parakeet recognition · Native Windows controls · Your voice stays here

</div>

Dictado Lite is being built as a small Windows dictation app: hold a shortcut to
speak, release it to insert text where you are writing. A quiet tray menu and a
tiny microphone pill will be its only everyday interface.

**Development status:** the native tray, hold/release shortcut, microphone capture,
resampling, cancellation and guarded Windows paste are implemented. Actual WAV
dictation into Notepad matches the previous Parakeet baseline. Conservative bilingual cleanup is implemented; the standalone installer is still in development;
no download is published. The tiny Direct2D pill now displays actual microphone
levels and recoverable errors; [visual decisions and coverage](docs/decisions/native-pill.md)
describe its native implementation and inspection captures.

![Actual native pill with quiet microphone input](assets/native-pill-listening.jpg)

The microphone drives the bars. On a paste failure, the result stays available:

![Actual recovery after a deliberate target focus switch](assets/native-pill-recovery.jpg)

These are isolated captures of the real executable in its developer inspection
shell mode. Normal use adds no taskbar window. See [measured UI checkpoint](benchmarks/native-pill-2026-10-08.md).

The planned app runs offline with one bundled Parakeet model (about 740 MB),
Vulkan acceleration and CPU fallback. It will not require Handy, Node, Python or
a browser runtime. Recognition quality and resource use still need measurement
in the complete app; see the measured [engine baseline](benchmarks/core-2026-10-08.md).
The native integration remains under physical Windows validation.

For contributors: [Build & contribute](CONTRIBUTING.md) ·
[Architecture](docs/architecture.md) · [Verification](docs/verification.md) ·
[Provenance](docs/provenance.md) · [AI guide](AGENTS.md).

Derived from Handy under [MIT](LICENSE). NVIDIA Parakeet weights use CC BY 4.0;
see [model attribution](models/manifest.json) and [third-party notices](THIRD_PARTY_NOTICES.md).
