# Control Tower

Control Tower is a local CLI workbench for ordered, user-owned executable stages. A workspace contains a `stages/` directory with numbered folders; each folder may contain executable `up`, `down`, `verify-up`, and `verify-down` files. Stage executables run with their stage folder as the working directory.

```text
my-workspace/
  stages/
    001-create-user/
      up
      down
      verify-up       # optional
      verify-down     # optional
```

Set up each local workspace explicitly before ordinary commands. This also adopts an existing v0 database without changing its saved checkpoint:

```sh
just db-bootstrap-local ./my-workspace
just db-migrate-local ./my-workspace
just db-verify-local ./my-workspace
```

Without `just`, run `cargo run -p control-tower-database --bin control-tower-db -- bootstrap-local ./my-workspace` and then the same command with `migrate-local` and `verify-local`. Normal CLI startup only opens the existing database and checks its version/history; it never creates tables or runs migrations.

Move forward or backward to a stage number (`0` is the baseline when moving down):

```sh
cargo run -p control-tower-cli -- up --workspace ./my-workspace --stage 1
cargo run -p control-tower-cli -- status --workspace ./my-workspace
cargo run -p control-tower-cli -- down --workspace ./my-workspace --stage 0
```

Control Tower runs each executable directly, honoring its executable bit and shebang. It sets `CONTROL_TOWER_WORKSPACE`, `CONTROL_TOWER_UUID`, `CONTROL_TOWER_STAGE`, `CONTROL_TOWER_DIRECTION`, and `CONTROL_TOWER_ROLE` in the child environment. The UUID stays the same across CLI invocations during a run and is cleared when the workspace returns to baseline. User scripts decide what that value means.

Control Tower preserves each executable's stdout, stderr, and exit status in its CLI report. A stage is recorded complete only after its mutation and optional directional verifier both succeed. A verifier failure leaves the transition pending so another invocation in the same direction retries verification without rerunning the mutation. Movement in the opposite direction runs that same stage's opposite mutation and optional verifier before continuing toward the target. Workbench state lives at `.control_tower/state.sqlite3` inside the workspace.

The [three-stage UUID-file workspace](examples/uuid-file) demonstrates the executable convention. Copy it before trying the full forward/backward walk so its generated file and SQLite state remain outside the repository:

```sh
workspace="$(mktemp -d)"
cp -R examples/uuid-file/. "$workspace/"
just db-bootstrap-local "$workspace"
just db-migrate-local "$workspace"
just db-verify-local "$workspace"
cargo run -p control-tower-cli -- up --workspace "$workspace" --stage 3
cargo run -p control-tower-cli -- down --workspace "$workspace" --stage 0
```
