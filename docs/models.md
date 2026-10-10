# Models

Setup offers the recommended **Parakeet TDT v3 Q8_0** download or a model picker.
The picker shows recommendations first and can expand to the full downloadable
catalog. The tray can reopen it later. Cancelling a download preserves the
previous selection. Models run locally and are kept separately from program files.

## Metadata

| File | Purpose |
| --- | --- |
| models/manifest.json | Default model, download pin and attribution |
| models/recommended.json | Initial recommendations |
| models/catalog.json | Full GGUF catalog snapshot inherited from Handy |

The catalog source is recorded in NOTICE. Each entry keeps its size, languages,
quantization, source and license metadata. Update model metadata and the native
engine together when a new architecture is needed. No model weights belong in
Git or installer payloads.

## Download and recognition

native/models.rs handles catalog lookup, cached models and verified HTTPS
downloads. native/platform/windows/models_ui.rs owns the picker. Downloads run
on a worker, validate their expected size and hash, and publish atomically.
Cancellation removes the partial file. Setup prepares the model before replacing
an existing installation.

Models with language detection receive no forced language hint. Other models use
a supported language derived from the Windows display language, with a fallback.
Output is not automatically translated. Available memory, languages and recognition
quality depend on the selected model; catalog inclusion does not guarantee every
model works on every PC.

Downloaded weights retain their own licenses. Use each model's source details
for its terms, especially entries marked "other". The application's MIT license
does not relicense model weights.
