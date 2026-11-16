## Dictado Lite 0.2.0

Cleaner text after pauses, a personal word list, and a terminal build that
also runs on Linux and macOS.

### What's new

- **Fewer stray periods:** pausing while you speak no longer leaves runs of
  periods or a period before the first word. A pause before a connecting word
  (y, porque, que, para, and, because, with...) no longer splits your sentence.
- **Your own words:** write `heard = written` lines in `replacements.txt`
  beside the settings (for example `hit have = GitHub`) and the recognized
  phrase is replaced, whole words only, case-insensitively. This is the way to
  fix English terms that a Spanish-first model spells phonetically.
- **Safer settings:** a shortcut saved by another version no longer stops the
  app from starting; the default shortcut is used instead.
- **Linux and macOS:** the same code builds on both. `dictado-lite --record`
  records from the microphone, recognizes locally and prints the text, so it
  can be piped into a clipboard tool. The tray application, the global
  shortcut and typing into other apps remain Windows only.
- **Checked before release:** formatting, lint and tests run on Linux and
  Windows in GitHub Actions for every change, and the release build repeats
  the Windows checks before packaging.

### Download

Download **Dictado-Lite-0.2.0-Setup.exe**, run it and choose your model.
Existing installations can be updated with the same setup. No administrator
rights are required. Settings and downloaded models survive uninstall.

Windows x64 is required for the installer. Vulkan acceleration is used when
available, with CPU fallback. The installer is currently unsigned. Application
code is MIT; downloaded models have their own licenses.
