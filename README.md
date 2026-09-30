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

Move forward or backward to a stage number (`0` is the baseline when moving down):

```sh
cargo run -p control-tower -- up --workspace ./my-workspace --stage 1
cargo run -p control-tower -- status --workspace ./my-workspace
cargo run -p control-tower -- down --workspace ./my-workspace --stage 0
```

Control Tower runs each executable directly, honoring its executable bit and shebang. It sets `CONTROL_TOWER_WORKSPACE`, `CONTROL_TOWER_UUID`, `CONTROL_TOWER_STAGE`, `CONTROL_TOWER_DIRECTION`, and `CONTROL_TOWER_ROLE` in the child environment. The UUID stays the same across CLI invocations during a run and is cleared when the workspace returns to baseline. User scripts decide what that value means.

Control Tower preserves each executable's stdout, stderr, and exit status in its CLI report. A stage is recorded complete only after its mutation and optional directional verifier both succeed. A verifier failure leaves the transition pending so another invocation in the same direction retries verification without rerunning the mutation. Workbench state lives at `.control_tower/state.sqlite3` inside the workspace.

The [three-stage UUID-file workspace](examples/uuid-file) demonstrates the executable convention. Copy it before trying the full forward/backward walk so its generated file and SQLite state remain outside the repository:

```sh
workspace="$(mktemp -d)"
cp -R examples/uuid-file/. "$workspace/"
cargo run -p control-tower -- up --workspace "$workspace" --stage 3
cargo run -p control-tower -- down --workspace "$workspace" --stage 0
```
