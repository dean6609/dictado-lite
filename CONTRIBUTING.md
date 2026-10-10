# Contributing

Use GitHub Flow: create a focused branch from current main, make the change,
verify it and open a pull request to dean6609/dictado-lite. Preserve unrelated
work and third-party attribution. Review the README and affected documentation
against the implementation, and record that review in the pull request.

## Build on Windows

Install Git and Rustup. The native engine needs a C++ compiler; the GNU build
script downloads its pinned tools, while MSVC builds use Visual Studio C++ tools.

```powershell
git clone https://github.com/dean6609/dictado-lite.git
cd dictado-lite
rustup toolchain install 1.99.0-x86_64-pc-windows-gnu --profile minimal --component rustfmt --component clippy
rustup override set 1.99.0-x86_64-pc-windows-gnu
./scripts/build-native.ps1 -Vulkan -Gnu
./scripts/check.ps1
./scripts/build-installer.ps1
```

Run these scripts in the same PowerShell session. The checks and installer build
need no model. Tools, compiled output and installer payloads are ignored by Git.
Never commit recordings, transcripts or downloaded model weights.

## Understand the code

[Architecture](docs/architecture.md) maps modules, threading and installation.
[Models](docs/models.md) explains metadata and downloads.
[AGENTS.md](AGENTS.md) gives coding assistants general guidance.

## Submit and merge

Use the pull request template. Describe the problem, resulting behavior,
verification and limitations. Inspect the real app for interface changes and use
actual captures. Disclose AI assistance and report only reviews actually performed.

Before merging, resolve review findings and verify the checks for the current
commit. Branch protection requires **Windows checks**, the single production
Windows build with format, lint, unit checks and installer packaging.
Prefer squash merge and delete the merged branch. Publish only to this project's
origin; the upstream Handy remote is for reference.

## Publish a release

Follow [the release checklist](docs/releasing.md). Merging a pull request does not
publish an installer: a matching version tag builds a draft release, which must
be checked and published. Keep the app version, tag and installer filename aligned.
