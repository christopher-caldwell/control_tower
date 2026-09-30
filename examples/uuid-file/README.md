# UUID file fixture

This workspace uses one UUID-named file throughout three ordered stages. Control Tower passes the same `CONTROL_TOWER_UUID` to every mutation and verifier. The scripts own the file contents and checks; Control Tower only runs them and tracks the stage position.

Run from the repository root, using a copy so the example stays clean:

```sh
workspace="$(mktemp -d)"
cp -R examples/uuid-file/. "$workspace/"
cargo run -p control-tower -- up --workspace "$workspace" --stage 3
cargo run -p control-tower -- down --workspace "$workspace" --stage 0
```

The stages progress through absent → empty → `hello` → `hello to you`; each down executable removes only its own contribution. The verifiers check the state after each directional mutation.
