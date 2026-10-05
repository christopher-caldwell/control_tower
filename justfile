# All operations require an explicit local workflow. There is no live database.
db-bootstrap-local workflow:
    cargo run --locked -p control-tower-cli -- db bootstrap-local '{{workflow}}'

db-migrate-local workflow:
    cargo run --locked -p control-tower-cli -- db migrate-local '{{workflow}}'

db-verify-local workflow:
    cargo run --locked -p control-tower-cli -- db verify-local '{{workflow}}'

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
