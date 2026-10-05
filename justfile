# All operations require an explicit local workflow. There is no live database.
db-bootstrap-local workflow:
    cargo run -p control-tower-database --bin control-tower-db -- bootstrap-local '{{workflow}}'

db-migrate-local workflow:
    cargo run -p control-tower-database --bin control-tower-db -- migrate-local '{{workflow}}'

db-verify-local workflow:
    cargo run -p control-tower-database --bin control-tower-db -- verify-local '{{workflow}}'

# Format every Rust crate in the workspace.
format:
    cargo fmt --all

# Check every Rust target and treat warnings as errors.
lint:
    cargo clippy --locked --workspace --all-targets -- -D warnings

# Build the workspace using the committed dependency lockfile.
build:
    cargo build --locked --workspace

# Install both commands required to use the workbench.
install-cli:
    cargo install --locked --path crates/cli --bin control-tower
    cargo install --locked --path crates/database --bin control-tower-db
