# Working in this repository

Read CONTRIBUTING.md and docs/architecture.md, then check the working tree,
branch and remotes. Use the architecture map to find the affected module and
follow its callers before editing.

Keep changes focused. Prefer the existing structure and abstractions. Keep
blocking work off interface and audio callbacks. Preserve unrelated changes,
user data and third-party attribution. Keep private inputs, downloaded models,
build tools and artifacts outside version control.

Use the build and check commands in CONTRIBUTING.md. Choose checks that exercise
the behavior being changed; do not add redundant tests or compatibility matrices.
Inspect actual application behavior for interface changes. State what was checked
and any unresolved limitation without adding permanent reports for routine work.

Update documentation when the behavior or structure changes. Review the diff,
open a pull request to the intended origin and follow the review/merge policy.
Keep guidance general; put implementation-specific metadata beside its owner.
