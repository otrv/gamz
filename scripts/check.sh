#!/usr/bin/env bash
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

step() {
    printf '==> %s\n' "$1"
}

unsafe_crates="game-memory"
miri_toolchain="nightly-2026-10-01"

step "cargo fmt"
cargo fmt --all -- --check

targets="$(awk -F'"' '/^targets[[:space:]]*=/ { for (i = 2; i < NF; i += 2) print $i }' rust-toolchain.toml)"
for target in $targets; do
    step "cargo clippy --target $target"
    cargo clippy --workspace --all-targets --all-features --locked --target "$target"
    step "cargo clippy (CLI only) --target $target"
    cargo clippy -p platform --no-default-features --features cli --all-targets --locked --target "$target"
done

step "cargo test (game-core and game-memory utilities)"
cargo test -p game-core -p game-memory --locked

rustup toolchain install "$miri_toolchain" --profile minimal --component miri,rust-src --no-self-update
for crate in $unsafe_crates; do
    step "miri ($miri_toolchain): $crate"
    cargo "+$miri_toolchain" miri test -p "$crate" --all-features --locked
done

step "all checks passed"
