# Provenance

Independent Windows dictation project derived from [Handy v0.9.8](https://github.com/cjpais/Handy/tree/v0.9.8), commit `14f6f0d31cb22a4268acfbddba3052820fd0a06e`. The starting clone was shallow; bootstrap fetched the missing history through that tag after GitHub rejected the incomplete object graph. Its Git history and MIT copyright notice are preserved. The original Handy installation and configuration are separate and must never be migrated implicitly.

The initial commit imports the upstream source, not a finished native application. Future extraction will record which audio, resampling, inference and insertion components are reused. Upstream workflows are removed at bootstrap so this independent repository cannot invoke Handy release infrastructure.

The selected model is NVIDIA Parakeet TDT 0.6B v3, converted to GGUF Q8_0 by handy-computer. Its revision, published LFS SHA-256 and size are recorded in `models/manifest.json`. The weights are not Git source. Attribution and license texts must accompany distribution.

MicFilter is a reference for documentation clarity and tray controls only. No GPL code or artwork is copied. Apple references inform proportions and motion only; no Apple assets are redistributed.
