---
id: CT-GUIDE-DEVELOPMENT
title: Development and contribution
type: guide
status: maintained
created: '2026-10-05'
updated: '2026-10-05'
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
