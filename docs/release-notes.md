## Dictado Lite 0.3.0

Ready-made packages for Linux and macOS, next to the Windows installer.

### What's new

- **Linux and macOS packages:** download the package for your system, unpack
  it and run `./install.sh`. It installs for your user only, in `~/.local`,
  and offers to download the recommended model. `dictado-lite --record`
  records from the microphone, recognizes locally and prints the text, so it
  can be piped into a clipboard tool. `./install.sh --uninstall` removes the
  program and keeps settings and models. The tray application, the global
  shortcut and typing into other apps remain Windows only.
- **Simpler model download in the terminal:** `dictado-lite --download-model`
  without a name fetches the model the settings select (the recommended one by
  default) and shows its progress.

### Download

- **Windows x64:** download **Dictado-Lite-0.3.0-Setup.exe**, run it and
  choose your model. Existing installations can be updated with the same
  setup. Vulkan acceleration is used when available, with CPU fallback.
- **Linux x64:** download **Dictado-Lite-0.3.0-Linux-x64.tar.gz**, unpack it
  and run `./install.sh`. Needs glibc 2.35 or newer (Ubuntu 22.04, Debian 12,
  Fedora 36 and later) and ALSA; recognition runs on the CPU.
- **macOS on Apple Silicon:** download
  **Dictado-Lite-0.3.0-macOS-arm64.tar.gz**, unpack it and run
  `./install.sh`. Recognition uses Metal.

No administrator rights are required. Settings and downloaded models survive
uninstall. No package is code-signed yet: Windows may show a SmartScreen
warning, and the macOS installer clears the download quarantine flag on its
own copy. Application code is MIT; downloaded models have their own licenses.
