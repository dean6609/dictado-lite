<div align="center">

![Dictado Lite — local voice, native Windows](assets/dictado-banner.svg)

**Speak. Release. Keep writing.**

Local Parakeet recognition · Native Windows controls · Your voice stays here

</div>

Dictado Lite is being built as a small Windows dictation app: hold a shortcut to
speak, release it to insert text where you are writing. A quiet tray menu and a
tiny microphone pill will be its only everyday interface.

**Development status:** repository and native Rust foundation only. The tray,
microphone, pill and standalone installer are not available yet. The inherited
Handy sources remain temporarily for extraction. No download is published.

The planned app runs offline with one bundled Parakeet model (about 740 MB),
Vulkan acceleration and CPU fallback. It will not require Handy, Node, Python or
a browser runtime. Recognition quality and resource use still need measurement
in the new executable; there is no claim of a finished or validated product.

For contributors: [Build & contribute](CONTRIBUTING.md) ·
[Architecture](docs/architecture.md) · [Verification](docs/verification.md) ·
[Provenance](docs/provenance.md) · [AI guide](AGENTS.md).

Derived from Handy under [MIT](LICENSE). NVIDIA Parakeet weights use CC BY 4.0;
see [model attribution](models/manifest.json) and [third-party notices](THIRD_PARTY_NOTICES.md).
