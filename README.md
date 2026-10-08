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
dictation into Notepad matches the previous Parakeet baseline. The visual pill,
full conservative cleanup and standalone installer are still in development;
no download is published.

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
