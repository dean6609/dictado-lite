<div align="center">

![Dictado Lite — local voice, native Windows](assets/dictado-banner.svg)

**Speak. Release. Keep writing.**

Local Parakeet recognition · Native Windows controls · Your voice stays here

</div>

Dictado Lite is a small Windows dictation app: hold **Ctrl+Alt+Space** to speak,
release it to insert text where you are writing. A quiet tray menu and a tiny
microphone pill are its everyday interface. **Escape** cancels.

## Install

[Download Windows x64 installer](https://github.com/dean6609/dictado-lite/releases/download/dictado-v0.1.0/Dictado-Lite-0.1.0-Setup.exe)
· [SHA-256](https://github.com/dean6609/dictado-lite/releases/download/dictado-v0.1.0/Dictado-Lite-0.1.0-Setup.exe.sha256)
· [Installation details](docs/installer.md).

Run the setup and open **Dictado Lite** from Start. The download is about 758 MiB
and includes the application, one Parakeet Q8_0 model and all notices. It works
offline with Vulkan acceleration and CPU fallback. No account, Handy, Node,
Python, CUDA or browser runtime is required.

![Actual native pill with quiet microphone input](assets/native-pill-listening.jpg)

The microphone drives the bars. On a paste failure, the result stays available:

![Actual recovery after a deliberate target focus switch](assets/native-pill-recovery.jpg)

These are isolated captures of the real executable in its developer inspection
shell mode. Normal use adds no taskbar window. See [measured UI checkpoint](benchmarks/native-pill-2026-10-08.md).

The tray controls dictate/stop, pause/resume, microphone, shortcut, optional basic
cleanup and optional Windows startup (off by default). There are no accounts,
model catalog or transcript history. Audio stays in memory during a session.
Cleanup uses conservative Spanish/English rules; it does not translate or guess
misrecognized words and can be switched off.

This initial release has passed native builds, WAV insertion/focus recovery,
installation and CPU fallback checks. Live spoken input, browser insertion and
some display/accessibility cases still need manual QA. See [verification](docs/verification.md)
and [measured resources](benchmarks/installed-app-2026-10-08.md).

## Build & modify

The complete source is available here. With Git and Rustup installed on Windows x64:

```powershell
git clone https://github.com/dean6609/dictado-lite.git
cd dictado-lite
rustup toolchain install 1.99.0-x86_64-pc-windows-gnu --profile minimal --component rustfmt --component clippy
rustup override set 1.99.0-x86_64-pc-windows-gnu
./scripts/build-native.ps1 -Vulkan -Gnu
./scripts/check.ps1
```

[CONTRIBUTING.md](CONTRIBUTING.md) explains model download, local execution,
installer creation and pull requests. Start an AI-assisted change with
[AGENTS.md](AGENTS.md): it maps the small engine, audio, Windows UI, cleanup and
setup modules and their checks. [Architecture](docs/architecture.md) explains how
they connect; [provenance](docs/provenance.md) records the original sources.

Derived from Handy under [MIT](LICENSE). NVIDIA Parakeet weights use CC BY 4.0;
see [model attribution](models/manifest.json) and [third-party notices](THIRD_PARTY_NOTICES.md).
