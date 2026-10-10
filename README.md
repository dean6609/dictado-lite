<div align="center">

<img src="docs/images/hero.svg" alt="Dictado Lite — Speak. Press. Keep writing." width="100%">

[![Download for Windows](https://img.shields.io/badge/Download_for_Windows-181B24?style=for-the-badge&logo=windows&logoColor=white)](https://github.com/dean6609/dictado-lite/releases/latest)
[![Latest release](https://img.shields.io/github/v/release/dean6609/dictado-lite?style=for-the-badge&color=818CF8&label=release)](https://github.com/dean6609/dictado-lite/releases/latest)

**Local voice typing, wherever you write.**

A small native Windows app that turns your voice into text in the focused editor.
No account. No subscription. No recordings or transcript history.

[Getting started](#getting-started) · [How it works](#how-it-works) · [Contributing](CONTRIBUTING.md)

</div>

## How it works

Place your cursor where you want to write. Press **Ctrl + Space**, speak, then
press **Ctrl + Space** again. Dictado Lite recognizes your speech on your PC,
inserts the text and dismisses the indicator. **Escape** cancels.

<p align="center">
  <img src="docs/images/recording.jpg" alt="Actual Dictado Lite recording indicator, with microphone activity bars" width="312">
  <br>
  <sub>The real recording indicator, enlarged for readability. Captured in inspection mode.</sub>
</p>

Release the keys while you speak. Change the shortcut, microphone or model from
the system tray. Previously saved shortcuts are kept when you upgrade.

## Getting started

1. Download **Dictado-Lite-0.1.1-Setup.exe** from the [latest release](https://github.com/dean6609/dictado-lite/releases/latest).
2. Choose the recommended model, or browse the downloadable models. Setup shows
   recommended choices first; the full catalog is one click away.
3. Open an editor and press **Ctrl + Space** to start.

The installer is approximately **55 MiB** and contains **no model weights**.
Setup downloads your chosen model once. Recognition works offline afterward.
Models are kept separately and reused across updates.

## Built to stay out of your way

- **On-device recognition.** Audio stays on your PC; processing runs locally.
  Internet is needed to download models from Hugging Face.
- **A native Windows interface.** A compact indicator, a tray menu and small
  settings dialogs. No web runtime or separate account to manage.
- **System language.** English and Spanish follow your Windows display language;
  other languages use English for now.
- **Your setup, your choice.** An editable shortcut, selectable microphone,
  optional text cleanup and optional startup with Windows.
- **Text recovery.** If insertion fails or the destination changes, copy the
  recognized result from the indicator or tray.

Requires **Windows x64**. Vulkan acceleration is used when available, with CPU
fallback. Memory, disk space, supported languages and speed depend on the model;
the recommended download is about **705 MiB**. Installation is per user and does
not require administrator rights. The installer is currently unsigned.

[Available models](docs/models.md)

## For contributors

Start with [CONTRIBUTING.md](CONTRIBUTING.md) for build commands and GitHub Flow.
The [architecture map](docs/architecture.md) explains the modules and application
flow. [AGENTS.md](AGENTS.md) provides general guidance for coding assistants.

## License and credits

Application code is [MIT licensed](LICENSE), derived from [Handy](https://github.com/cjpais/Handy),
with the native transcribe.cpp / ggml engine. Original attribution is retained in
[NOTICE](NOTICE) and [third-party notices](THIRD_PARTY_NOTICES.md).

Downloaded models have their own licenses. The recommended NVIDIA Parakeet v3
model is CC BY 4.0; handy-computer provides its GGUF conversion. Model details
link to each source and its terms. Model weights are not bundled in the installer.
