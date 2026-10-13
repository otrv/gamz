#!/usr/bin/env bash
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

step() {
    printf '==> %s\n' "$1"
}

targets=(x86_64-unknown-linux-gnu)
unsafe_crates="game-memory"
miri_toolchain="nightly-2026-10-01"
deny_version="0.20.2"

step "cargo fmt"
cargo fmt --all -- --check

step "cargo deny ($deny_version)"
if [[ "$(cargo deny --version 2>/dev/null || true)" != "cargo-deny $deny_version" ]]; then
    cargo install cargo-deny --version "=$deny_version" --locked
fi
cargo deny --workspace --locked check --deny warnings bans
step "cargo deny (asset-packer: no dependencies)"
cargo deny --manifest-path crates/asset-packer/Cargo.toml --config crates/asset-packer/deny.toml --locked check --deny warnings bans

selections=$(cargo metadata --locked --format-version 1 --no-deps | jq -r '
    .packages[] | select(.name == "game-entry") | .features | keys[] | select(. != "default")
')

for target in "${targets[@]}"; do
    if ! rustup target list --installed | grep -Fx "$target" >/dev/null; then
        rustup target add "$target"
    fi
    step "cargo clippy --target $target"
    cargo clippy --workspace --exclude platform --exclude game-entry --exclude examples --all-targets --all-features --locked --target "$target"
    cargo clippy -p renderer --no-default-features --all-targets --locked --target "$target"
    for selection in $selections; do
        for feature in cli linux cli,linux; do
            step "cargo clippy ($selection, $feature) --target $target"
            cargo clippy -p platform --no-default-features --features "$feature,game-entry/$selection" --all-targets --locked --target "$target"
        done
    done
done

step "cargo test (game-core and game-memory utilities)"
cargo test -p game-core -p game-memory --locked

if ! rustup run "$miri_toolchain" cargo miri --version >/dev/null 2>&1 \
    || ! rustup component list --toolchain "$miri_toolchain" --installed | grep -Fx rust-src >/dev/null; then
    rustup toolchain install "$miri_toolchain" --profile minimal --component miri,rust-src --no-self-update
fi
for crate in $unsafe_crates; do
    step "miri ($miri_toolchain): $crate"
    cargo "+$miri_toolchain" miri test -p "$crate" --all-features --locked
done

step "all checks passed"
