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
cargo build --locked --release
./scripts/stage-runtime.ps1
./scripts/build-installer.ps1
```

Run these scripts in the same PowerShell session. The installer build needs no
model. Tools, compiled output and installer payloads are ignored by Git.
Everything builds locally; forks need no GitHub Actions or repository access.
Never commit recordings, transcripts or downloaded model weights.

## Build on Linux or macOS

Install Rustup, CMake and a C++ compiler (GCC or Clang; Xcode command line
tools on macOS). Linux also needs the ALSA headers (`libasound2-dev` on
Debian and Ubuntu, `alsa-lib-devel` on Fedora). Downloads use the system `curl`.

```sh
git clone https://github.com/dean6609/dictado-lite.git
cd dictado-lite
cargo build --locked --release
./target/release/dictado-lite --download-model handy-computer/parakeet-tdt-0.6b-v3-gguf/parakeet-tdt-0.6b-v3-Q8_0.gguf
./target/release/dictado-lite --record
```

The first build compiles the engine from the crate sources, which takes a few
minutes. The result is one static binary: CPU recognition on Linux, Metal with
CPU fallback on macOS. These systems get the terminal commands only; the tray
application, global shortcut and text insertion are Windows code.

## Understand the code

[Architecture](docs/architecture.md) maps modules, threading and installation.
[Models](docs/models.md) explains metadata and downloads.
[AGENTS.md](AGENTS.md) gives coding assistants general guidance.

## Submit and merge

Use the pull request template. Describe the problem, resulting behavior,
verification and limitations. Inspect the real app for interface changes and use
actual captures. Disclose AI assistance and report only reviews actually performed.

Before merging, review the diff and resolve findings. For code changes, run
`./scripts/check.ps1` on Windows for format, lint, existing units and
compilation; on Linux or macOS run `cargo fmt --all --check`,
`cargo clippy --locked --all-targets -- -D warnings` and `cargo test --locked`.
The **Build** workflow runs the same on Linux and Windows for every pull
request and must be green before merging; macOS is checked only on request
(`check_macos` when running the workflow by hand), because GitHub bills its
minutes at ten times the Linux rate. For installer changes,
`./scripts/smoke-installer.ps1` exercises a separate QA installation and
removal. Documentation-only changes need no application build.
Prefer squash merge and delete the merged branch. Publish only to this project's
origin; the upstream Handy remote is for reference.

## Publish a release

Follow [the release checklist](docs/releasing.md). Merging a pull request does not
publish an installer. Launch the installer workflow manually from `main`, with
an existing version tag, to run the checks and create a draft for inspection
and publication. Tools and the native engine are cached on `main` for reuse
between versions. Commits, PRs and tags launch no automatic installer build.
Keep the app version, tag and installer filename aligned.
