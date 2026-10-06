# Control Tower

Control Tower is a **local development and testing workbench for repeatable stateful scenarios**. It helps developers prepare a known state, exercise an application change, inspect what happened, and move the scenario backward to try again. You define the executable actions and checks; Control Tower records workflow position and runs them in order. It is a focused workbench, not a general-purpose workflow or orchestration engine.

## Install

Clone the repository and install the CLI onto your `PATH` with either supported source-based path:

```sh
git clone https://github.com/christopher-caldwell/control_tower.git
cd control_tower
```

With `just` installed, this installs `control-tower` and its agent Skill:

```sh
just install-cli
```

Or install only the CLI with Cargo:

```sh
cargo install --locked --path crates/cli --bin control-tower
```

## Ways to Use Control Tower

### UI

The loopback browser UI is primarily for human use. From a Workspace root, run `control-tower ui` and open the local URL it prints. Use the interface to inspect stages and choose a movement.

### CLI

The CLI is primarily for agent use and works just as well for people. From a Workspace root, select a Workflow explicitly:

```sh
control-tower validate --workflow workflows/uuid-file
control-tower status --workflow workflows/uuid-file
control-tower up --workflow workflows/uuid-file --stage 3
control-tower down --workflow workflows/uuid-file --stage 0
```

For installation prerequisites, a complete UI or CLI walkthrough, and the existing UUID-file example, follow the [canonical getting-started guide](docs/guides/getting-started.md).

## Help and contribution

The [documentation index](docs/README.md) links to command references, authoring details, examples, and troubleshooting. Contributor and UI development instructions are in [Development and contribution](docs/guides/development.md).
