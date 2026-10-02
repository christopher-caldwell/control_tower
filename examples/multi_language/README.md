# Multi-language example family

**Control Tower does not have a runtime. Individual stages do.** These independent
workspaces compose ordinary languages through explicit files. Their stage files
show the runtime boundary directly.

| Workspace | Prerequisites beyond Control Tower and a Unix shell | Progression |
| --- | --- | --- |
| [shell-python-node](workspaces/shell-python-node/README.md) | uv / Python 3.12, Node / npm | Shell input → Python normalization → Node inspection. |
| [go-rust](workspaces/go-rust/README.md) | Go 1.23+, Rust/Cargo and a linker | Go input → Rust transformation, invoked with ordinary toolchain commands. |

Copy an individual workspace directory. Each owns all language metadata, locks,
source, and stage files needed for its scenario. This family has no shared runtime
project or reusable tool layer.

Run `./examples/test --family multi_language` from the repository root. Unavailable
optional toolchains are reported as skips; compilation or execution failures are
failures. Return to the [gallery](../README.md).
