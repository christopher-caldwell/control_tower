---
id: CT-GUIDE-DEVELOPMENT
title: Development and contribution
type: guide
status: maintained
created: '2026-10-05'
updated: '2026-10-07'
owner: christopher-caldwell
authored_by: assistant
sources:
- ../../Cargo.toml
- ../../justfile
- ui-development.md
---

# Development and contribution

Work from a source checkout. For Rust or documentation changes, use the repository's Rust workspace and keep user-facing documentation under `docs/` with the canonical [getting-started walkthrough](getting-started.md) as the human entry path. Check links, terminology, command examples, and current implementation behavior together when changing guides.

The Rust workspace checks are:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --locked --workspace
```

For React development, custom Workspace setup, browser tests, and rebuilding embedded assets, follow the [UI development guide](ui-development.md). The [documentation index](../README.md) links to maintained product, authoring, and reference docs; dated research/history records preserve their original evidence and terminology.

## Prepare a source release

Install the version tool once:

```sh
cargo install cargo-edit --locked --no-default-features --features set-version
```

From the repository root, run:

```sh
just publish        # patch by default
just publish minor
just publish major
```

The recipe calls [`publish.sh`](../../publish.sh), which installs the locked UI
dependencies, builds `ui/dist/`, uses
[`cargo-edit`](https://github.com/killercup/cargo-edit#cargo-set-version) to bump
only `control-tower-cli` and update `Cargo.lock`, then builds the release executable
at `target/release/control-tower`. From the repository root, you can also run
`./publish.sh [patch|minor|major]` directly. Invoke the script by path to use it
from another working directory.

Run the relevant checks above before preparing the release. Review and commit the
generated assets, `crates/cli/Cargo.toml`, and `Cargo.lock`, then push. This command
prepares local release files; committing, tagging, pushing, and registry publication
remain separate operations.
