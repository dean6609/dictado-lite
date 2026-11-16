# Native architecture

Dictado Lite is a Rust core with a native C++ recognition engine and no web
runtime. On Windows it is a tray application; on Linux and macOS the same core
builds a terminal binary. Model weights are downloaded separately.

| Location | Responsibility |
| --- | --- |
| native/main.rs | Entry point, terminal commands (`--record`, file recognition, downloads) |
| native/session.rs | Active-session state and rejection of cancelled results |
| native/engine/ | Native model ownership, recognition worker and cancellation |
| native/audio/ | Microphone capture and conversion to mono 16 kHz audio |
| native/cleanup.rs | Mechanical text cleanup and the user's word list |
| native/paths.rs | Per-user settings and model cache folder on each system |
| native/transfer.rs | One HTTPS download (WinHTTP on Windows, curl elsewhere) |
| native/platform/windows/ | Tray, shortcut, indicator, settings and text insertion |
| native/models.rs | Catalog, selected model and verified downloads |
| native/setup/ | Windows setup wizard, owned-file installation and removal |
| scripts/ | Build tools, checks, Windows installer and Linux/macOS packages |
| models/ | Catalog snapshot, recommendation list and default model metadata |
| licenses/ | Original dependency license texts required for distribution |

## Application flow

The first shortcut press starts recording; the second stops capture and submits
recognition. Escape cancels. The UI thread owns Windows resources. Audio capture
and inference work outside it; blocking downloads run on a worker. Session IDs
prevent late or cancelled results from being inserted.

Audio stays in memory. Recognition produces text for the originally focused
editor. Insertion requires an unchanged destination and released modifiers.
Clipboard contents are restored only while still owned by the app; a newer user
copy is preserved. Failed insertion retains text for explicit recovery.

The indicator does not take focus. Its microphone bars use actual input levels.
The tray controls shortcut, microphone, model, optional cleanup and startup.
The model unloads after idle time. Hidden UI does not run an animation timer.

`dictado-lite --record` is the same capture, recognition and cleanup without
a window: it records until Enter or a time limit and prints the text. It is the
whole user interface on Linux and macOS.

## Text cleanup

Cleanup is mechanical and never translates or guesses words. It removes
repeated sentence marks left by pauses, rejoins a sentence the recognizer split
before a connecting word, drops a few unambiguous fillers and repeated
fragments, and applies the user's `replacements.txt` (`heard = written`, one
per line, whole words, case-insensitive). Text that looks like code is left
untouched. The word list is applied only when cleanup is enabled.

## Installation and settings

Setup downloads or reuses the selected model before replacing an installation.
New installers contain no weights. Partial downloads never become active models.
Upgrade and uninstall operate on manifest-owned files and preserve unrelated files.

Program files live in %LOCALAPPDATA%/Programs/DictadoLite on Windows and in
~/.local/lib/dictado-lite (command linked from ~/.local/bin) when the Linux or
macOS package is installed with install.sh. Settings, the word
list and models live in the per-user data folder and survive uninstall:
%LOCALAPPDATA%/DictadoLite on Windows, ~/Library/Application Support/DictadoLite
on macOS and $XDG_DATA_HOME/dictado-lite (default ~/.local/share/dictado-lite)
on Linux. Handy installations and caches are separate. See
[model metadata](models.md) and [release steps](releasing.md).

## Engine builds

Windows links a prebuilt shared engine with loadable backends (CPU tiers and
Vulkan) produced by scripts/build-native.ps1. Linux and macOS compile the engine
statically from the crate sources through CMake during `cargo build`: CPU on
Linux, Metal with CPU fallback on macOS. The GPU request is Metal on macOS and
Vulkan elsewhere; any GPU failure retries once on the CPU. Release packages
never tune the engine for the build machine: scripts/package-unix.sh builds
Linux with shared libraries and per-tier CPU modules found through an
`$ORIGIN/lib` rpath, and macOS as the same static Metal binary.
