# Native architecture

Dictado Lite is a Windows tray application written in Rust with a native C++
recognition engine. It has no web runtime. Model weights are downloaded separately.

| Location | Responsibility |
| --- | --- |
| native/main.rs | Entry point, command-line options and configuration |
| native/session.rs | Active-session state and rejection of cancelled results |
| native/engine/ | Native model ownership, recognition worker and cancellation |
| native/audio/ | Microphone capture and conversion to mono 16 kHz audio |
| native/platform/windows/ | Tray, shortcut, indicator, settings and text insertion |
| native/models.rs | Catalog, selected model and verified downloads |
| native/setup/ | Setup wizard, owned-file installation and removal |
| scripts/ | Build tools, checks and packaging |
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

## Installation and settings

Setup downloads or reuses the selected model before replacing an installation.
New installers contain no weights. Partial downloads never become active models.
Upgrade and uninstall operate on manifest-owned files and preserve unrelated files.

Program files live in %LOCALAPPDATA%/Programs/DictadoLite. Settings and models live
in %LOCALAPPDATA%/DictadoLite and survive uninstall. Handy installations and caches
are separate. See [model metadata](models.md) and [release steps](releasing.md).
