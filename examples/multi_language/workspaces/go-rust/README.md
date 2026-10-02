# Go and Rust

Go writes `21` to a file. Rust reads it and persists the doubled value, `42`.
The executable shell roles use ordinary `go run ./main.go` and `cargo run` commands
to compile and invoke the programs at execution time. Control Tower executes those
role files directly, just as it executes a shell, Python, or Node role. Source lives
beside the corresponding stage. Shell verifiers inspect the files; reversal removes
only the file produced by that stage.

## Prerequisites and progression

Use a Unix shell, built Control Tower binaries, and Go 1.23+, Rust / Cargo, and a working linker.
The workspace is independent of Control Tower's Cargo workspace through its own
`[workspace]` declaration. Go and Rust use their standard libraries; no module
registry dependencies are needed. Generated Cargo output stays under its ignored
`target/` directory; Go uses its normal build cache.

```text
001 shell -> go run ./main.go -> data/number.txt
002 shell -> cargo run --locked -> data/doubled.txt
```

## Run the workspace

Follow [example setup](../../README.md#setup-and-run), then run from the copied example root:

```sh
workspace=workspaces/go-rust
# No third-party language dependencies; Cargo.lock is checked in.
# The first up command compiles using your installed toolchains.
control-tower-db bootstrap-local "$workspace"
control-tower-db migrate-local "$workspace"
control-tower-db verify-local "$workspace"
control-tower up --workspace "$workspace" --stage 1
cat "$workspace/data/number.txt"
control-tower up --workspace "$workspace" --stage 2
cat "$workspace/data/doubled.txt"
control-tower status --workspace "$workspace"
control-tower down --workspace "$workspace" --stage 0
control-tower status --workspace "$workspace"
```

Expected file contents are `21` and `42`, each followed by a newline.
Reversing to baseline removes stage-owned outputs and
clears the active UUID; the checkpoint database and installed environments/build
caches remain. Each `down` removes only its named output files. Keep unrelated data
away from these paths: reversal does not restore overwritten files.

If a mutation fails after writing a file, inspect its partial output before
retrying. These deterministic transformations can overwrite their own outputs on
retry. After a verifier failure, repeating the same move retries only verification.
For the global rules, see [navigation and verification](https://github.com/christopher-caldwell/control_tower/blob/feat/examples-gallery/docs/guides/verification-and-navigation.md)
and the [executable contract](https://github.com/christopher-caldwell/control_tower/blob/feat/examples-gallery/docs/reference/stage-executables.md).
Return to the [example](../../README.md).
