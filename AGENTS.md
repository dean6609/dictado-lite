# Guide for coding assistants

## What this project is

Dictado Lite is a small native Windows app for local voice typing: press a
shortcut, speak, and the text is typed into the app the user was working in.
It is an independent, reduced derivative of Handy, rewritten as a native Rust
core with a C++ recognition engine.

## Intentions

These are the qualities every change should protect:

- **Local and private.** Audio never leaves the PC and is never stored. The
  network is used only to download models.
- **Lightweight.** Native code, a small installer, no web runtime, no bundled
  model weights and near-zero cost while idle.
- **Simple.** Few features, done well. Prefer improving or removing over adding;
  a new setting needs a clear reason.
- **Careful with the user's work.** Never type into the wrong place, never lose a
  result, never overwrite the user's clipboard, settings or files.
- **Easy to fork.** Anyone can clone, build and package on their own computer.
  Nothing should depend on access to this repository or its services.

## Where to look

| Need | Read |
| --- | --- |
| What users see and expect | README.md |
| Build, checks and pull requests | CONTRIBUTING.md |
| Modules, threads and data flow | docs/architecture.md |
| Model metadata and downloads | docs/models.md |
| Publishing a release | docs/releasing.md |

## Keep context small

Source files are small and focused on one responsibility. Find the module in the
architecture map and read it and its callers, not the whole tree. When a file
grows well past a few hundred lines or mixes responsibilities, split it.

Skip large vendored or generated files unless the task is about them:
licenses/, models/catalog.json and Cargo.lock. Never explore the ignored local
folders .tools/, target/, artifacts/ and local/: they hold toolchains, build
output and private notes, tens of thousands of files.

## How to work

Check the working tree, branch and remotes first. Keep changes focused, follow
the existing structure, and keep blocking work off interface and audio callbacks.
Preserve unrelated changes, user data and third-party attribution. Keep
recordings, downloaded models, build tools and artifacts out of Git.

Use the checks in CONTRIBUTING.md that exercise what changed; do not add
redundant tests. Inspect the real app for interface changes. Update these guides
when behavior or structure changes, keeping them general: intentions here,
specifics beside the code that owns them. Report what was checked and any open
limitation in the pull request, not in new files.
