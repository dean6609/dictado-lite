# Working in this repository

Start with CONTRIBUTING.md and docs/architecture.md. Check the working tree,
current branch and remotes before editing or publishing changes.

## Find the owner of a change

- Application entry points and orchestration live in `native/`.
- Recognition and worker lifetime live in `native/engine/`.
- Recording and sample preparation live in `native/audio/`.
- Platform integration and interface code live in `native/platform/`.
- Installation and removal live in `native/setup/`.
- Build and packaging commands live in `scripts/`.
- Download metadata lives in `models/`; original notices live in `licenses/`.
- Explanations, contracts and verification evidence live in `docs/`.

Follow the affected module's callers and tests before changing its contract.
Keep related behavior together and isolate blocking work from interface and audio
callbacks. Prefer existing abstractions; add dependencies for concrete needs.

## Implement and verify

Work on a focused branch from current main. Preserve unrelated changes and user
data. Keep private inputs, transcripts, downloads and build artifacts outside
version control. Validate external input before publishing files or results.

Use the commands in CONTRIBUTING.md. Test meaningful behavior and failure paths;
compilation alone does not prove an interface or installation works. For visible
changes, inspect the real application and capture reviewable evidence. Describe
what was checked and what still needs physical verification.

Update affected documentation to match the implementation. Review the diff,
retain applicable third-party attribution, and open a pull request using the
repository template. Publish only to the intended project remote. Verify checks
for the current PR commit and follow the repository's review/merge policy.
