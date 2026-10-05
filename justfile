# All operations require an explicit local workflow. There is no live database.
db-bootstrap-local workflow:
    #!/usr/bin/env sh
    set -eu
    workflow_path="$(cd "{{workflow}}" && pwd)"
    workspace="$(dirname "$(dirname "$workflow_path")")"
    manifest="$(pwd)/Cargo.toml"
    cd "$workspace"
    cargo run --locked --manifest-path "$manifest" -p control-tower-cli -- db bootstrap-local --workflow "$workflow_path"

db-migrate-local workflow:
    #!/usr/bin/env sh
    set -eu
    workflow_path="$(cd "{{workflow}}" && pwd)"
    workspace="$(dirname "$(dirname "$workflow_path")")"
    manifest="$(pwd)/Cargo.toml"
    cd "$workspace"
    cargo run --locked --manifest-path "$manifest" -p control-tower-cli -- db migrate-local --workflow "$workflow_path"

db-verify-local workflow:
    #!/usr/bin/env sh
    set -eu
    workflow_path="$(cd "{{workflow}}" && pwd)"
    workspace="$(dirname "$(dirname "$workflow_path")")"
    manifest="$(pwd)/Cargo.toml"
    cd "$workspace"
    cargo run --locked --manifest-path "$manifest" -p control-tower-cli -- db verify-local --workflow "$workflow_path"

# Format every Rust crate in the workspace.
format:
    cargo fmt --all

# Check every Rust target and treat warnings as errors.
lint:
    cargo clippy --locked --workspace --all-targets -- -D warnings

# Build the workspace using the committed dependency lockfile.
build:
    cargo build --locked --workspace

# Develop the UI with hot reload and a prepared sample workspace.
dev-ui:
    pnpm --dir ui dev

# Install the CLI and skills into ~/.agents/skills (or ~/.claude/skills).
[positional-arguments]
install-cli skills-dir="agents":
    #!/usr/bin/env sh
    set -eu
    case "$1" in
        agents|claude) destination="$HOME/.$1/skills" ;;
        *) echo "error: skills-dir must be agents or claude" >&2; exit 1 ;;
    esac
    cargo install --locked --path crates/cli --bin control-tower
    mkdir -p "$destination"
    cp -R skills/. "$destination/"
