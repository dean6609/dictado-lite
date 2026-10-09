# Provenance

Independent Windows dictation project derived from [Handy v0.9.8](https://github.com/cjpais/Handy/tree/v0.9.8), commit `14f6f0d31cb22a4268acfbddba3052820fd0a06e`. The starting clone was shallow; bootstrap fetched the missing history through that tag after GitHub rejected the incomplete object graph. Its Git history and MIT copyright notice are preserved. The original Handy installation and configuration are separate and must never be migrated implicitly.

The initial commit imported upstream source; the current native application removes
Tauri/web/cloud/history. Its engine comes from `transcribe-cpp-sys` 0.2.4,
upstream commit `4807edaf210d0d7e8a6f7fb2a44b65966a2797f0`, archive SHA-256
`1c6946c7bf90046fc51e5bc25373a4f6270ca10fdcf4f35b1174075e43c535b5`.
Version 0.1.0 selected Parakeet-only sources. Version 0.1.1 restores the full
architecture registry for the Handy GGUF catalog and keeps only the missing
standard `<functional>` header fix for libc++. Rust ownership/abort handling
adapts MIT transcribe-cpp patterns. Audio and guarded insertion are described in
architecture.md. This project has its own release workflows and remote.

The catalog snapshot comes from Handy commit
`f6b3f8297061acaa763a48c4cdefbbf342c814ac`; its exact source and MIT ownership
are recorded in NOTICE. All 69 entries retain their original model metadata.

The selected model is NVIDIA Parakeet TDT 0.6B v3, converted to GGUF Q8_0 by handy-computer. Its revision, published LFS SHA-256 and size are recorded in `models/manifest.json`. The weights are not Git source. Attribution and license texts must accompany distribution.

MicFilter is a reference for documentation clarity and tray controls only. No GPL code or artwork is copied. Apple references inform proportions and motion only; no Apple assets are redistributed.
