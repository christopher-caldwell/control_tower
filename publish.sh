#!/usr/bin/env sh
# Prepare the embedded UI and CLI for a source release.
set -eu

bump="${1-patch}"
if [ "$#" -gt 1 ]; then
    printf 'Usage: %s [patch|minor|major]\n' "$0" >&2
    exit 2
fi
case "$bump" in
    patch|minor|major) ;;
    *) printf 'Usage: %s [patch|minor|major]\n' "$0" >&2; exit 2 ;;
esac

cd "$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"

if ! cargo set-version --help >/dev/null 2>&1; then
    printf 'Install version tooling: cargo install cargo-edit --locked --no-default-features --features set-version\n' >&2
    exit 1
fi

pnpm --dir ui install --frozen-lockfile
pnpm --dir ui build
cargo set-version --package control-tower-cli --bump "$bump"
cargo build --locked --release -p control-tower-cli

printf 'Release prepared. Review and commit ui/dist/, crates/cli/Cargo.toml, and Cargo.lock before pushing.\n'
