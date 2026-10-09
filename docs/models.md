# Model catalog and download ownership

`models/catalog.json` vendors the GGUF catalog from Handy commit
`f6b3f8297061acaa763a48c4cdefbbf342c814ac`; `NOTICE` records its original MIT
ownership. It contains 69 entries across the native engine's architecture
families. This is a snapshot, not a promise to track Handy's future models live.

`native/models.rs` reads metadata and manages verified downloads.
`native/platform/windows/models_ui.rs` owns the native picker and progress window.
The setup downloads the recommended model or presents selection within the same
window, before installation and download. Runtime missing-model recovery still
uses the standalone picker.
The tray can reopen the picker later. Selection changes only after successful
download and verification; cancelling preserves the active model and settings.

Each entry retains its quantization, size, languages, license and source metadata.
The UI starts with four recommended models and exposes the full catalog on request.
models/recommended.json records tested multilingual alternatives independently
from the unchanged upstream catalog.
Licensing stays in documentation and the optional source details, not setup copy.
The recommended model remains Parakeet TDT v3 Q8_0; `models/manifest.json` preserves
its exact attribution. Cache filenames include the hash prefix to prevent source
collisions. Models with automatic language detection receive no language hint;
others receive a supported fallback. Output is never automatically translated.

`scripts/build-native.ps1` builds the full pinned transcribe.cpp archive, with only
the build-header compatibility fix in `scripts/prepare-native.ps1`. The previous
Parakeet-only modifications are retained in Git history. Review the catalog and
native engine versions together when updating either.

Licenses marked `other` must be inspected on the source model page. Downloading
weights does not relicense them under MIT. Keep model-specific restrictions and
attribution separate from the application license. No weights enter Git or setup.

Compiler coverage includes all registered families. Recognition checks use the
recommended model and representative alternatives; they do not prove every
catalog model, language, GPU or memory configuration. Large models may exceed a
particular computer's available memory.
